// literal type.
// (transitional; will eventually replace with JsVal)

pub enum Literal {
    Number(f64),
    String(~str),
    Boolean(bool),
    Null,
    Undefined
}
