use super::command::Commands;
use super::error::Errors;
use super::arch::Arch;

/// this parser parses the packet on a surface level
/// and passes on the resulting data to a packet struct
/// which knows how to parse and execute itself and
/// generate a response if required
/// the parser and commands are designed to avoid
/// allocations wherever possible
/// To generate a response string you will also need to provide
/// a mutable slice of bytes to write to
/// the command responder will write to that slice until it is filled at which point
/// it will return a MemoryFilledInterupt error
/// and the client program can provide a new buffer to write to after handling the current
/// packet slice
/// When the response is fully written it will return an Ok
/// T is a custom context type for the out of memory handler
pub struct Parser<'a> {
    packet: &'a [u8],
    current: usize,

}

impl<'a> Parser<'a> {
    pub fn new(packet: &'a [u8]) -> Self {
        Self {
            packet,
            current: 0
        }
    }

    // packet layout:
    // $<optional id:>packet-data#checksum
    // if this function causes an error
    // a retransmit packet should be sent
    pub fn parse<T>(&mut self) -> Result<Commands<'a, T>, Errors>
    where T: Arch {
        // first char needs to be $
        if !self.is_match(b'$') {
            // bail
            return Err(Errors::UnexpectedIntroduction);
        }



        Ok(Commands::Unsupported)
    }

    pub fn advance(&mut self) -> u8 {
        self.current += 1;
        *self.packet.get(self.current).unwrap_or(&b'\0')
    }

    pub fn peek(&self) -> u8 {
        *self.packet.get(self.current).unwrap_or(&b'\0')
    }

    pub fn next(&self) -> u8 {
        *self.packet.get(self.current+1).unwrap_or(&b'\0')
    }

    pub fn is_match(&mut self, c: u8) -> bool {
        if self.peek() == c {
            self.advance();
            true
        } else {
            false
        }
    }

    // verifies that checksum is ok
    pub fn checksum(&self, cs: &[u8]) -> bool {
        false
    }

    pub fn parse_checksum(&self) -> &str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
