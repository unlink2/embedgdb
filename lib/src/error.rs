
#[derive(Debug, Eq, PartialEq)]
pub enum Errors {
    MemoryFilledInterupt,
    NotTerminated,
    InvalidChecksum,
    UnexpectedIntroduction,
}
