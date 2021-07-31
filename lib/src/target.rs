/// This is the cpu architecture specific
/// This is the cpu architecture specific
/// response data and io handling
/// The target context should be cheap to clone!
pub trait Target: Clone + PartialEq {

    /// returns the halt reason
    /// as a slice of bytes
    fn reason(&self) -> &[u8] {
        b"SAA"
    }

    fn registers(&self) -> &[u8] {
        b"xxxxxxxxxxxxxxxx"
    }
}
