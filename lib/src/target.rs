/// This is the cpu architecture specific
/// response data and io handling
/// The target context should be cheap to clone!
pub trait Target: Clone + PartialEq {
    /// This function is called whenever the provided memory buffer
    /// is not sufficient
    /// it allows the handling of the memory buffer;
    /// return true if the buffer has been handeled (e.g. transmitted)
    /// or false to abort and attempt with a larger buffer
    /// T is used to provide a custom context for handling the data
    /// this can be nearly any object
    fn buffer_full(&mut self, response_data: &[u8]) -> bool;

    /// returns the halt reason
    /// as a slice of bytes
    fn reason(&self) -> &[u8] {
        b"SAA"
    }
}
