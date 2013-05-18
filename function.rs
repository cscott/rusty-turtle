// function type.
use op::Op;

pub struct Function {
    id: int,
    nargs: int,
    max_stack: int,
    bytecode: ~[uint]
}
