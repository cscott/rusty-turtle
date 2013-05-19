// javascript object implementation
use function::Function;
use intern::{Interner,IString};

// this describes the fields in the object map.
// we use some fields for internal implementation details (like the Function
// associated with a function object) which we want to hide from the user.
struct FieldDesc {
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
    map: ObjectMap
}

// this is an ordered list of fields, which label the fields
// in the JsObjectValue.fields vector.  The 'children' list collects
// all the maps built from this one, with exactly one more field,
// which allows us to unify identical maps.
struct ObjectMap {
    fields: ~[FieldDesc],
    children: ~[FDOM]
}
impl ObjectMap {
    fn find(&self, desc: FieldDesc) -> Option<uint> {
        for self.fields.eachi |i, f| {
            if *f == desc { return Some(i); }
        }
        return None;
    }
    fn new() -> ~ObjectMap {
        ~ObjectMap { fields: ~[], children: ~[] }
    }
    fn with_field<'r>(&'r mut self, desc: FieldDesc) -> &'r ObjectMap {
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
                    map: ObjectMap {
                        fields: (self.fields + ~[desc]),
                        children: ~[]
                    }
                };
                pos = self.children.len();
                self.children.push(fdom);
            }
        }
        &'r (self.children[pos].map)
    }
}

// an object is a combination of a map (which labels the fields) and
// the actual values of the fields (
pub struct Object {
    map: @ObjectMap,
    fields: @[JsVal]
}
impl Object {
    fn get_simple(self, desc: FieldDesc) -> Option<JsVal> {
        match self.map.find(desc) {
            None => None,
            Some(idx) => Some(self.fields[idx])
        }
    }
    // IString(0) should always correspond to __proto__
    fn get(self, desc: FieldDesc) -> JsVal {
        match self.get_simple(desc) {
            Some(val) => val,
            None => match self.get_simple(FieldDesc::proto()) {
                Some(JsObject(parent)) => parent.get(desc),
                _ => { return JsUndefined; }
            }
        }
    }
/*
    fn set(@mut self, desc: FieldDesc, val: JsVal) {
        match self.map.find(desc) {
            Some(idx) => { self.fields[idx] = val; },
            None => {
                // need to add this to the map
                self.map = self.map.with_field(desc);
                // now add to the object's field vector
                self.fields = self.fields + ~[val];
            }
        }
    }
*/
}

pub enum JsVal {
    JsObject(Object),
    JsNumber(f64),
    JsString(@str),
    JsUndefined,
    JsNull,
    // not visible to user code
    JsFunctionCode(@Function)
}
