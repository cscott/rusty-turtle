// literal type.

pub enum Literal {
    Number(f64),
    String(~str),
    Boolean(bool),
    Null,
    Undefined
}
