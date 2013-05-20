use function::Function;
use object::{JsVal,JsNumber,JsBool,JsUndefined,JsNull};

use startup_init = startup::init;

// utility structure
priv struct Reader {
    buf : ~[u8],
    pos : uint
}
impl Reader {
    fn new(buf : ~[u8]) -> Reader {
        Reader { buf : buf, pos : 0 }
    }
    fn decode_uint(&mut self) -> uint {
        let val = self.buf[self.pos] as uint;
        self.pos += 1;
        if val < 128 { return val; }
        (val - 128u) + (128u * self.decode_uint())
    }
    fn decode_str(&mut self) -> ~str {
        let len = self.decode_uint();
        let mut utf16 : ~[u16] = vec::with_capacity(len);
        while vec::len(utf16) < len {
            vec::push(&mut utf16, self.decode_uint() as u16);
        }
        str::from_utf16(utf16)
    }
}

// this represents a compilation unit (which can be as small as a function)
pub struct Module {
    functions: ~[Function],
    literals: ~[JsVal]
}

impl Module {
    pub fn new_startup_module() -> Module {
        let mut functions : ~[Function] = ~[];
        let mut literals : ~[JsVal] = ~[];
        startup_init(&mut functions, &mut literals);
        Module { functions: functions, literals: literals }
    }

    pub fn new_from_bytes(buf : ~[u8]) -> Module {
        let mut reader = Reader::new(buf);
        // parse functions
        let num_funcs = reader.decode_uint();
        let mut functions : ~[Function] = vec::with_capacity(num_funcs);
        let mut func_id = 0;
        while func_id < num_funcs {
            let nargs = reader.decode_uint();
            let max_stack = reader.decode_uint();
            let name = reader.decode_str();
            let blen = reader.decode_uint();
            let mut bytecode : ~[uint] = vec::with_capacity(blen);
            while vec::len(bytecode) < blen {
                vec::push(&mut bytecode, reader.decode_uint());
            }
            vec::push(&mut functions, Function {
                name: if str::is_empty(name) { None } else { Some(name) },
                id: func_id,
                nargs: nargs,
                max_stack: max_stack,
                bytecode: bytecode
            });
            func_id += 1;
        }
        // parse literals
        let num_lits = reader.decode_uint();
        let mut literals : ~[JsVal] = vec::with_capacity(num_lits);
        while vec::len(literals) < num_lits {
            let l = match reader.decode_uint() {
                0 => { // number tag
                    let num = reader.decode_str();
                    if "Infinity" == num { // xxx rust doesn't allow commutative
                        JsNumber(f64::infinity)
                    } else if "-Infinity" == num {
                        JsNumber(f64::neg_infinity)
                    } else {
                        match f64::from_str(num) {
                            Some(f) => JsNumber(f),
                            _ => fail!()
                        }
                    }
                },
                1 => JsVal::from_str(reader.decode_str()), // string tag
                2 => JsBool(true), // boolean tags
                3 => JsBool(false),
                4 => JsNull,
                5 => JsUndefined,
                _ => fail!()
            };
            vec::push(&mut literals, l);
        }
        Module { functions: functions, literals: literals }
    }
}
