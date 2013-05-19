use op::Op;
use function::Function;
use literal::Literal;
use module::Module;
use object::{Object,JsVal};


struct State {
    // main interpreter state.
    parent: Option<State>, // calling context (another state)
    frame: ~Object,
    stack: ~[JsVal],
    pc: uint,
    // from bytecode file
    module: Module,
    func_id: uint,
    // cached
    bytecode: ~[uint],
    literals: ~[Literal]
}
