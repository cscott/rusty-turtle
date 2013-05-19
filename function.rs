// function type.

pub struct Function {
    name: Option<~str>,
    id: uint,
    nargs: uint,
    max_stack: uint,
    bytecode: ~[uint]
}
