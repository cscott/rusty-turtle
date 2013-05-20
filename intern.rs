// a simple string interner.
// tries to do it "the rust way" which means the user can put the
// interner wherever they want, can have multiple interners, etc.
use core::hashmap::HashMap;
use core::local_data::{local_data_get,local_data_set};

pub struct IString {
    priv id : uint
}
impl TotalEq for IString {
    fn equals(&self, other: &IString) -> bool { self.id == other.id }
}
impl Eq for IString {
    fn eq(&self, other: &IString) -> bool { self.id == other.id }
    fn ne(&self, other: &IString) -> bool { self.id != other.id }
}
impl IString {
    fn to_str(self, interner : &Interner) -> ~str {
        interner.reverse_map[self.id].clone()
    }
    fn to_uint(self, interner: &Interner) -> Option<uint> {
        // XXX more efficient implementation possible in the future
        let s = self.to_str(interner);
        uint::from_str(s)
    }
    // for reserved words
    pub fn zero() -> IString { IString { id: 0 } }
}

priv struct Interner {
    priv map : @mut HashMap<~str, IString>,
    priv reverse_map : @mut ~[~str]
}
impl Interner {
    pub fn new() -> Interner {
        Interner {
            map: @mut HashMap::new(),
            reverse_map: @mut ~[]
        }
    }

    pub fn prefill(init : &[&str]) -> Interner {
        let i = Interner::new();
        for init.each() |v| { i.intern(*v); }
        i
    }

    pub fn intern(&self, s : &str) -> IString {
        // xxx note that we have to clone s to make a ~str from a &str
        //     since s may be inserted into the map
        let rv = do self.map.find_or_insert_with(s.to_str()) |s| {
            let is = IString { id: self.reverse_map.len() };
            self.reverse_map.push(s.clone());
            is
        };
        *rv
    }

    // convenience function
    pub fn get(&self, is : IString) -> ~str { is.to_str(self) }
}

// use a task-local interner
priv fn interner_key(_x: @Interner) { }

priv fn get_task_local_interner() -> @Interner {
    unsafe {
        let interner = match local_data_get(interner_key) {
            Some(val) => val,
            None => {
                let data = @Interner::prefill(["__proto__"]);
                local_data_set(interner_key, data);
                data
            }
        };
        interner
    }
}

pub fn intern(s:&str) -> IString {
    get_task_local_interner().intern(s)
}

pub fn intern_get(is:IString) -> ~str {
    is.to_str(get_task_local_interner())
}

pub fn intern_to_uint(is:IString) -> Option<uint> {
    is.to_uint(get_task_local_interner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_fail]
    fn i1() {
        let i = Interner::new();
        i.get(IString { id: 13 });
    }
    #[test]
    fn i2 () {
        // local interner
        let i = Interner::prefill(["__proto__"]);
        // first one is one:
        assert_eq!(i.intern (~"dog"), IString { id: 1 });
        // re-use gets the same entry:
        assert_eq!(i.intern (~"dog"), IString { id: 1 });
        // different string gets a different #:
        assert_eq!(i.intern (~"cat"), IString { id: 2 });
        assert_eq!(i.intern (~"cat"), IString { id: 2 });
        // dog is still at one
        assert_eq!(i.intern (~"dog"), IString { id: 1 });
        // new string gets 3
        assert_eq!(i.intern (~"zebra" ), IString { id: 3 });
        assert_eq!(i.get(IString { id: 1 }), ~"dog");
        assert_eq!(i.get(IString { id: 2 }), ~"cat");
        assert_eq!(i.get(IString { id: 3 }), ~"zebra");
        // __proto__ is zero
        assert_eq!(i.intern (~"__proto__"), IString::zero());
        // IString comparison
        assert_eq!(i.intern (~"rhino"), i.intern (~"rhino"));
    }
    #[test]
    fn i3 () {
        // task-global interner
        // first one is one:
        assert_eq!(intern (~"dog"), IString { id: 1 });
        // re-use gets the same entry:
        assert_eq!(intern (~"dog"), IString { id: 1 });
        // different string gets a different #:
        assert_eq!(intern (~"cat"), IString { id: 2 });
        assert_eq!(intern (~"cat"), IString { id: 2 });
        // dog is still at one
        assert_eq!(intern (~"dog"), IString { id: 1 });
        // new string gets 3
        assert_eq!(intern (~"zebra" ), IString { id: 3 });
        assert_eq!(intern_get(IString { id: 1 }), ~"dog");
        assert_eq!(intern_get(IString { id: 2 }), ~"cat");
        assert_eq!(intern_get(IString { id: 3 }), ~"zebra");
        // __proto__ is zero
        assert_eq!(intern (~"__proto__"), IString::zero());
        // IString comparison
        assert_eq!(intern (~"rhino"), intern (~"rhino"));
    }
    #[test]
    fn i4() {
        // "as uint" methods
        let is1 = intern("dog");
        assert_eq!(intern_to_uint(is1), None);
        let is2 = intern("3");
        assert_eq!(intern_to_uint(is2), Some(3u));
        let is3 = intern("-4");
        assert_eq!(intern_to_uint(is3), None);
        let is4 = intern("010");
        assert_eq!(intern_to_uint(is4), Some(10u));
        let is5 = intern("a");
        assert_eq!(intern_to_uint(is5), None);
    }
}
