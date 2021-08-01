use super::error::Errors;
use super::stream::Stream;
use super::parser::Parser;

/// This is the cpu architecture specific
/// This is the cpu architecture specific
/// response data and io handling
/// The target context should be cheap to clone!
pub trait Target {

    /// returns the halt reason
    /// as a slice of bytes
    fn reason(&self) -> &[u8] {
        b"S05" // sigtrap
    }

    fn rd_registers(&self, stream: &mut dyn Stream) -> Result<(), Errors> {
        Ok(())
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
        let mut registers = [0; 38];

        // set PC to reset vector
        registers[37] = 0xBFC00000;
        Self {
            memory: [0; 512],
            registers
        }
    }
}

impl Target for VirtualTarget {
    fn rd_registers(&self, stream: &mut dyn Stream) -> Result<(), Errors> {
        for reg in self.registers {
            Parser::to_hex32(&reg.to_le_bytes(), stream)?;
        }
        Ok(())
    }

    fn wr_registers(&mut self, data: &[u8]) -> Result<(), Errors> {
        // is there enough data?
        if data.len() % 8 != 0 || data.len() != self.registers.len() * 8 {
            Err(Errors::CommandError)
        } else {
            let c = data.chunks(8);
            for (i, bytes) in c.enumerate() {
                let mut b: [u8; 8] = Default::default();
                b.copy_from_slice(bytes);
                b.reverse();
                match Parser::from_hex32(&b) {
                    Some(value) => self.registers[i] = value,
                    _ => return Err(Errors::CommandError)
                }
            }
            Ok(())
        }
    }
}
