#[derive(Debug, Eq, PartialEq)]
pub enum Errors {
    MemoryFilledInterupt,
    NotTerminated,
    InvalidChecksum,
    UnexpectedIntroduction,
    CommandError,
    OutOfDataError,
    BadNumber,
    InsufficientArguments,
    AddressOutOfRange,
    LengthMismatch,
}
