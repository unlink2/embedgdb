use super::error::Errors;
use super::parser::Parser;
use super::stream::Stream;
use crate::parser::Endianness;

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

    fn rd_registers(&self, _stream: &mut dyn Stream) -> Result<usize, Errors> {
        Ok(0)
    }

    /// write to registers
    fn wr_registers(&mut self, _data: &[u8]) -> Result<usize, Errors> {
        Ok(0)
    }

    /// reads memory
    /// evil raw pointers are being used to represent the start address!
    fn rd_memory(
        &self,
        _start: *const u8,
        _size: usize,
        _stream: &mut dyn Stream,
    ) -> Result<usize, Errors> {
        Ok(0)
    }

    /// write to registers
    /// evil raw pointers are being used to represent the start address!
    fn wr_memory(&mut self, _start: *const u8, _data: &[u8]) -> Result<usize, Errors> {
        Ok(0)
    }

    fn endianess(&self) -> Endianness {
        Endianness::Little
    }
}

/// This is a demo implementation
/// simulating a mips cpu
pub struct VirtualTarget {
    registers: [u32; 38],
    memory: [u8; 512],
}

impl VirtualTarget {
    pub fn new() -> Self {
        let mut registers = [(1 as u32).to_be(); 38];

        // set PC to reset vector
        registers[37] = (0xBFC00000 as u32).to_be();
        Self {
            memory: [0; 512],
            registers,
        }
    }
}

impl Target for VirtualTarget {
    fn endianess(&self) -> Endianness {
        Endianness::Big
    }

    fn rd_registers(&self, stream: &mut dyn Stream) -> Result<usize, Errors> {
        let stream_start = stream.pos();
        for reg in self.registers {
            Parser::to_hexu(&reg.to_be_bytes(), stream)?;
        }
        Ok(stream.pos() - stream_start)
    }

    fn wr_registers(&mut self, data: &[u8]) -> Result<usize, Errors> {
        // is there enough data?
        if data.len() % 8 != 0 || data.len() != self.registers.len() * 8 {
            Err(Errors::CommandError)
        } else {
            let c = data.chunks(8);
            for (i, bytes) in c.enumerate() {
                match Parser::from_hexu(&bytes) {
                    Some(value) => self.registers[i] = value as u32,
                    _ => return Err(Errors::CommandError),
                }
            }
            Ok(0)
        }
    }

    fn rd_memory(
        &self,
        start: *const u8,
        size: usize,
        stream: &mut dyn Stream,
    ) -> Result<usize, Errors> {
        let start = usize::min(start as usize, self.memory.len()); // for virtual target don't use real pointers
        let end = usize::min(start as usize + size, self.memory.len());

        let stream_start = stream.pos();
        for byte in self.memory[start..end].iter() {
            Parser::to_hex8(byte.to_be(), stream)?;
        }
        Ok(stream.pos() - stream_start)
    }

    fn wr_memory(&mut self, start: *const u8, data: &[u8]) -> Result<usize, Errors> {
        let start = start as usize; // for virtual target don't use real pointers
        let end = start + data.len();

        if start >= self.memory.len() || end >= self.memory.len() {
            Err(Errors::AddressOutOfRange)
        } else {
            let c = data.chunks(2);
            for (i, data) in c.enumerate() {
                let byte = match Parser::from_hexu(data) {
                    Some(value) => value as u8,
                    _ => return Err(Errors::CommandError),
                };
                self.memory[i + start] = byte;
            }
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Command, SupportedCommands};
    use crate::parser::Parsed;
    use crate::stream::BufferedStream;
    use crate::target::VirtualTarget;

    struct DebugCommands;
    impl<'a> SupportedCommands<'a> for DebugCommands {}

    fn exec_packet(
        result: &mut Parsed,
        rstream: &mut BufferedStream,
        target: &mut VirtualTarget,
    ) -> Result<usize, Errors> {
        assert_ne!(result.response, None);
        assert_ne!(result.command, None);
        let mut size = 0;
        if let Some(response) = &mut result.response {
            size += response.response(rstream, target)?;
        }

        if let Some(command) = &mut result.command {
            size += command.response(rstream, target)?;
        }

        return Ok(size);
    }

    #[test]
    fn it_should_execute_read_register() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$g#67");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let size = exec_packet(&mut result, &mut rstream, &mut target).unwrap();

        assert_eq!(size, 309);
        assert_eq!(rstream.buffer[..rstream.pos()],
            b"$010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000000c0bf#c0"[..]);
    }

    #[test]
    fn it_should_execute_wr_register() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$Gd1bccabf0100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000100000001000000010000000000c0bf#6c");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let size = exec_packet(&mut result, &mut rstream, &mut target).unwrap();

        assert_eq!(size, 7);
        assert_eq!(rstream.buffer[..rstream.pos()], b"$OK#9a"[..]);
    }

    #[test]
    fn it_should_read_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$m64,4#37");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let size = exec_packet(&mut result, &mut rstream, &mut target).unwrap();

        assert_eq!(size, 13);
        assert_eq!(rstream.buffer[..rstream.pos()], b"$00000000#80"[..]);
    }

    #[test]
    fn it_should_read_partial_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$m1fe,4#c9");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let size = exec_packet(&mut result, &mut rstream, &mut target).unwrap();

        assert_eq!(size, 9);
        assert_eq!(rstream.buffer[..rstream.pos()], b"$0000#c0"[..]);
    }

    #[test]
    fn it_should_reject_insufficient_args_read_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$m64#d7");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let err = exec_packet(&mut result, &mut rstream, &mut target).unwrap_err();
        assert_eq!(err, Errors::InsufficientArguments);
    }

    #[test]
    fn it_should_write_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$M64,4:ab000000#34");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let size = exec_packet(&mut result, &mut rstream, &mut target).unwrap();

        assert_eq!(size, 7);
        assert_eq!(rstream.buffer[..rstream.pos()], b"$OK#9a"[..]);
    }

    #[test]
    fn it_should_reject_size_mismatch_write_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$M64,3:ab000000#33");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let error = exec_packet(&mut result, &mut rstream, &mut target).unwrap_err();

        assert_eq!(error, Errors::LengthMismatch);
    }

    #[test]
    fn it_should_reject_insufficient_args_write_memory() {
        let mut target = VirtualTarget::new();
        let mut parser = Parser::new(b"$M64,3#16");
        let mut rstream = BufferedStream::new();

        let mut result = parser.parse_packet(&DebugCommands);
        let error = exec_packet(&mut result, &mut rstream, &mut target).unwrap_err();

        assert_eq!(error, Errors::InsufficientArguments);
    }
}
