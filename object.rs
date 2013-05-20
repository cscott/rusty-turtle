// javascript object implementation
use function::Function;
use intern::IString;

// this describes the fields in the object map.
// we use some fields for internal implementation details (like the Function
// associated with a function object) which we want to hide from the user.
pub struct FieldDesc {
    name: IString,
    hidden: bool
}
impl Eq for FieldDesc {
    fn eq(&self, other: &FieldDesc) -> bool {
        self.hidden == other.hidden && self.name == other.name
    }
    fn ne(&self, other: &FieldDesc) -> bool {
        self.hidden != other.hidden || self.name != other.name
    }
}
impl FieldDesc {
    // corresponds to __proto__ field
    fn proto() -> FieldDesc {
        FieldDesc { name: IString::zero(), hidden: false }
    }
}

// utility tuple
priv struct FDOM {
    field: FieldDesc,
    map: @mut ObjectMap
}

// this is an ordered list of fields, which label the fields
// in the JsObjectValue.fields vector.  The 'children' list collects
// all the maps built from this one, with exactly one more field,
// which allows us to unify identical maps.
pub struct ObjectMap {
    fields: ~[FieldDesc],
    mut children: ~[FDOM]
}
impl ObjectMap {
    fn find(&self, desc: FieldDesc) -> Option<uint> {
        for self.fields.eachi |i, f| {
            if *f == desc { return Some(i); }
        }
        return None;
    }
    pub fn new() -> ObjectMap {
        ObjectMap { fields: ~[], children: ~[] }
    }
    fn with_field(&mut self, desc: FieldDesc) -> @mut ObjectMap {
        assert_eq!(self.find(desc), None);
        let pos : uint;
        match self.children.position(|fdom| { fdom.field == desc }) {
            Some(p) => {
                pos = p;
            },
            None => {
                // hm, have to create a new one.
                let fdom = FDOM {
                    field: desc,
                    map: @mut ObjectMap {
                        fields: (self.fields + ~[desc]),
                        children: ~[]
                    }
                };
                pos = self.children.len();
                self.children.push(fdom);
            }
        }
        self.children[pos].map
    }
}

// an object is a combination of a map (which labels the fields) and
// the actual values of the fields (
pub struct Object {
    map: @mut ObjectMap,
    mut fields: ~[JsVal]
}
impl Object {
    pub fn new(root_map: &mut ObjectMap) -> @mut Object {
        @mut Object {
            map: root_map.with_field(FieldDesc::proto()),
            fields: ~[JsNull]
        }
    }

    // the root is the singleton returned by Object::new()
    pub fn create(root_map: &mut ObjectMap, parent: @mut Object) -> @mut Object {
        @mut Object {
            map: root_map.with_field(FieldDesc::proto()),
            fields: ~[JsObject(parent)]
        }
    }

    pub fn contains_simple(&self, desc: FieldDesc) -> bool {
        match self.map.find(desc) {
            None => false,
            Some(_) => true
        }
    }

    pub fn get_simple(&self, desc: FieldDesc) -> Option<JsVal> {
        match self.map.find(desc) {
            None => None,
            Some(idx) => Some(self.fields[idx])
        }
    }

    // IString(0) should always correspond to __proto__
    pub fn contains(&self, desc: FieldDesc) -> bool {
        if self.contains_simple(desc) {
            true
        } else {
            match self.get_simple(FieldDesc::proto()) {
                Some(JsObject(parent)) => parent.contains(desc),
                _ => false
            }
        }
    }

    // IString(0) should always correspond to __proto__
    pub fn get(&self, desc: FieldDesc) -> JsVal {
        match self.get_simple(desc) {
            Some(val) => val,
            None => match self.get_simple(FieldDesc::proto()) {
                Some(JsObject(parent)) => parent.get(desc),
                _ => { return JsUndefined; }
            }
        }
    }

    // XXX implement more efficient storage/intern for 'number-like'
    //     field names.
    pub fn set(&mut self, desc: FieldDesc, val: JsVal) {
        match self.map.find(desc) {
            Some(idx) => { self.fields[idx] = val; },
            None => {
                // need to add this to the map
                self.map = self.map.with_field(desc);
                // now add to the object's field vector
                // xxx: improve O(N) copy here
                self.fields = self.fields + ~[val];
            }
        }
    }
}

pub type NativeFunction = @fn(JsVal, ~[JsVal]) -> JsVal;

pub enum JsVal {
    JsObject(@mut Object),
    JsNumber(f64),
    JsString(@[u16]),
    JsUndefined,
    JsNull,
    // not visible to user code
    JsFunctionCode(@Function),
    JsNativeFunction(NativeFunction)
}
impl JsVal {
    pub fn to_str(self) -> ~str {
        match(self) {
            JsObject(_) => ~"[object]",
            JsNumber(n) => n.to_str(),
            JsString(utf16) => str::from_utf16(utf16),
            JsUndefined => ~"undefined",
            JsNull => ~"null",
            JsFunctionCode(_) => ~"[function]", // xxx use f.name
            JsNativeFunction(_) => ~"[native function]"
        }
    }
    pub fn from_str(s: &str) -> JsVal {
        JsString(at_vec::to_managed_consume(str::to_utf16(s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern::intern;

    #[test]
    fn test_object_fields() {
        let root_map = @mut ObjectMap::new();
        let fdType = FieldDesc { name: intern("type"), hidden: true };

        let myObject = Object::new(root_map); // parent of all objects.
        myObject.set(fdType, JsVal::from_str("object"));

        let myArray = Object::create(root_map, myObject);
        myArray.set(fdType, JsVal::from_str("array"));
        myArray.set(FieldDesc { name: intern("length"), hidden: false },
                    JsNumber(0f64));

        assert!( myObject.contains(fdType) );
        assert!( myArray.contains(fdType) );
        assert_eq!(myObject.get(fdType).to_str(), ~"object");
        assert_eq!(myArray.get(fdType).to_str(), ~"array");
        assert_eq!(myArray.get(FieldDesc { name: intern("length"),
                                          hidden: false }).to_str(), ~"0");

        // test prototypal inheritance
        let fdFoo = FieldDesc { name: intern("foo"), hidden: false };

        assert!( !myObject.contains(fdFoo) );
        assert!( !myArray.contains(fdFoo) );
        assert_eq!(myObject.get(fdFoo).to_str(), ~"undefined");
        assert_eq!(myArray.get(fdFoo).to_str(), ~"undefined");

        myObject.set(fdFoo, JsVal::from_str("bar"));

        assert_eq!(myArray.get(fdFoo).to_str(), ~"bar");
        assert_eq!(myObject.get(fdFoo).to_str(), ~"bar");
        assert!( myObject.contains(fdFoo) );
        assert!( myArray.contains(fdFoo) );

        // test that hidden fields stay separate
        let fdHiddenFoo = FieldDesc { name: intern("foo"), hidden: true };
        assert!( !myArray.contains(fdHiddenFoo) );
        assert_eq!(myArray.get(fdHiddenFoo).to_str(), ~"undefined");

        myArray.set(fdHiddenFoo, JsNumber(42f64));
        assert!( myArray.contains(fdHiddenFoo) );
        assert_eq!(myArray.get(fdHiddenFoo).to_str(), ~"42");
        assert_eq!(myArray.get(fdFoo).to_str(), ~"bar");
    }
}
