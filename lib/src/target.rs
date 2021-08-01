use super::error::Errors;

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
    fn wr_registers(&mut self, _data: &[u8]) -> Result<(), Errors> {
        Ok(())
    }
}

/// This is a demo implementation
/// simulating a mips cpu
pub struct VirtualTarget {
    registers: [u32; 38],
    memory: [u8; 512]
}

impl VirtualTarget {
    pub fn new() -> Self {
        Self {
            registers: [0; 38],
            memory: [0; 512]
        }
    }
}

impl Target for VirtualTarget {
}
