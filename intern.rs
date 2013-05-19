// a simple string interner.
// tries to do it "the rust way" which means the user can put the
// interner wherever they want, can have multiple interners, etc.
use core::hashmap::HashMap;

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
    // for reserved words
    pub fn zero() -> IString { IString { id: 0 } }
}

struct Interner {
    priv map : HashMap<~str, IString>,
    priv reverse_map : ~[~str]
}
impl Interner {
    pub fn new() -> ~Interner {
        ~Interner {
            map: HashMap::new(),
            reverse_map: ~[]
        }
    }

    pub fn prefill(init : &[&str]) -> ~Interner {
        let mut i = Interner::new();
        for init.each() |v| { i.intern(*v); }
        i
    }

    pub fn intern(&mut self, s : &str) -> IString {
        // xxx note that we have to clone s to make a ~str from a &str
        *do self.map.find_or_insert_with(s.to_str()) |s| {
            let is = IString { id: self.reverse_map.len() };
            self.reverse_map.push(s.clone());
            is
        }
    }
    // convenience function
    pub fn get(&self, is : IString) -> ~str { is.to_str(self) }
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
        let mut i = Interner::prefill(["__proto__"]);
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
}
