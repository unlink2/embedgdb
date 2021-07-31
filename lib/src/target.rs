/// This is the cpu architecture specific
/// This is the cpu architecture specific
/// response data and io handling
/// The target context should be cheap to clone!
pub trait Target {

    /// returns the halt reason
    /// as a slice of bytes
    fn reason(&self) -> &[u8] {
        b"SAA"
    }

    fn rd_registers(&self) -> &[u8] {
        // fake mips registers
        &[b'x'; 304]
    }

    /// write to registers
    fn wr_registers(&mut self, _data: &[u8]) {
    }
}

/// This is a demo implementation
/// simulating a mips cpu
pub struct VirtualTarget {
}

impl VirtualTarget {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Target for VirtualTarget {
}
