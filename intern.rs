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
        let mut i = ~Interner {
            map: HashMap::new(),
            reverse_map: ~[]
        };
        i.intern(~"__proto__"); // ensure this is IString(0)
        i
    }
    pub fn intern(&mut self, s : ~str) -> IString {
        *do self.map.find_or_insert_with(s) |s| {
            let is = IString { id: self.reverse_map.len() };
            self.reverse_map.push(s.clone());
            is
        }
    }
}

/*
pub fn foo(a: IString, b: IString) -> bool { a==b }
pub fn bar(a: uint, b: uint) -> bool { a==b }
fn main() {
    let mut a = Interner::new();
    let b = a.intern(~"foo");
    let c = a.intern(~"bar");
    let d = foo(b, c);
    debug!(d);
}
*/
