use super::command::*;
use super::error::Errors;
use super::target::Target;

// Holds the Acknowledge Packet and command packet

#[derive(Debug, PartialEq)]
pub struct Parsed<'a, T>
where T: Target {
    pub response: Option<Commands<'a, T>>,
    pub command: Option<Commands<'a, T>>
}

impl<'a, T> Parsed<'a, T>
where T: Target {
    fn new(response: Option<Commands<'a, T>>, command: Option<Commands<'a, T>>) -> Self {
        Self {response, command}
    }
}

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

    fn retransmit<T>(ctx: T, error: Errors) -> Parsed<'a, T>
    where T: Target {
        Parsed::new(
            Some(Commands::Retransmit(Retransmit::new(ctx, error))), None)
    }

    fn not_implemented<T>(ctx: T) -> Parsed<'a, T>
    where T: Target {
        Parsed::new(
            Some(Commands::Acknowledge(Acknowledge::new(ctx.clone()))),
            Some(Commands::NotImplemented(NotImplemented::new(ctx))))
    }

    // packet layout:
    // $<optional id:>packet-data#checksum
    // if this function causes an error
    // a retransmit packet should be sent
    pub fn parse_packet<T>(&mut self, ctx: T) -> Parsed<T>
    where T: Target {
        // first char needs to be $
        if !self.is_match(b'$') {
            // bail
            return Self::retransmit(ctx, Errors::UnexpectedIntroduction);
        }

        // read packet name
        // packet names either are terminated by #, space, comma or semicolon
        let name = self.parse_token();

        // read rest of data, those will be parsed when the packet is interpreted/executed
        let _ = self.parse_until_end();

        // read end-delim
        if !self.is_match(b'#') {
            // retransmit - the packet never terminated!
            return Self::retransmit(ctx, Errors::NotTerminated);
        }

        // is checksum ok?
        if !self.verify_chksm(self.packet) {
            return Self::retransmit(ctx, Errors::InvalidChecksum);
        }

        match name {
            _ => Self::not_implemented(ctx)
        }
    }

    pub fn parse_token(&mut self) -> &'a [u8] {
        let start = self.current;
        while !self.is_at_end()
            && !self.is_term() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    pub fn parse_until_end(&mut self) -> &'a [u8] {
        let start = self.current;
        while !self.is_match(b'#')
            && !self.is_at_end() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    pub fn is_term(&self) -> bool {
        match self.peek() {
            b' ' | b',' | b'#' | b';' => true,
            _ => false,
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.current > self.packet.len()
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
    pub fn verify_chksm(&mut self, cs: &[u8]) -> bool {
        // read next 2 bytes
        let b0 = Self::from_hex(self.advance());
        let b1 = Self::from_hex(self.advance());

        if let (Some(b0), Some(b1)) = (b0, b1) {
            let n0 = Self::from_hex(b0);
            let n1 = Self::from_hex(b1);

            if let (Some(n0), Some(n1)) = (n0, n1) {
                // now we have a sum, calculate based on data and see!
                let sum = (n1 << 4) & n0;

                let calc = Self::chksm8(self.packet);

                sum != calc
            } else {
                false
            }
        } else {
            // in all other cases bail with bad checksum!
            false
        }
    }

    // adds a buffe rto chksm
    pub fn chksm(response_data: &[u8]) -> u32 {
        // never include $ and # in checksum. -> this is fine because they always need
        // to be escaped anyway so in a well-formed packet they should always appear at the
        // start/end
        let mut chksm = 0;
        'cloop: for b in response_data {
            match *b {
                b'$' => (),
                b'#' => break 'cloop,
                _ => chksm += *b as u32
            }
        }

        chksm % 256
    }

    pub fn chksm8(response_data: &[u8]) -> u8 {
        (Self::chksm(response_data) % 256) as u8
    }

    pub fn is_digit(b: u8) -> bool {
        b >= b'0' && b <= b'9'
    }

    pub fn is_hex(b: u8) -> bool {
        (b >= b'a' && b <= b'f')
            || (b >= b'A' && b <= b'F')
    }

    pub fn is_hex_digit(b: u8) -> bool {
        Self::is_hex(b) || Self::is_digit(b)
    }

    pub fn from_hex(b: u8) -> Option<u8> {
        if b >= b'0' && b <= b'9' {
            Some(b - b'0')
        } else if b >= b'A' && b <= b'F' {
            Some(b - b'A')
        } else if b >= b'a' && b <= b'f' {
            Some(b - b'A')
        } else {
            None
        }
    }

    pub fn to_hex(b: u8) -> Option<u8> {
        if b >= 16 {
            None
        } else if b <= 9 {
            Some(b + b'0')
        } else {
            Some(b + b'A' - 10)
        }
    }

    pub fn to_hex_tuple(b: u8) -> (u8, u8) {
        let h = (b >> 4) & 0xF;
        let l = b & 0xF;
        // we can unwrap here because it will always be valid
        (Self::to_hex(h).unwrap(), Self::to_hex(l).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestCtx;
    impl Target for TestCtx {
        fn on_mem_filled(&mut self, response_data: &[u8]) -> bool {
            false
        }
    }

    #[test]
    fn it_should_parse_hex() {
        assert_eq!(Parser::to_hex(15), Some(b'F'));
        assert_eq!(Parser::to_hex(6), Some(b'6'));
        assert_eq!(Parser::to_hex(16), None);
    }

    #[test]
    fn it_should_parse_hex_tupel() {
        assert_eq!(Parser::to_hex_tuple(0xA7), (b'A', b'7'));
    }

    #[test]
    fn it_should_parse_packet() {
        let chksm = "$vMustReplyEmpty#3a".as_bytes();

        let mut parser = Parser::new(chksm);
        let ctx = TestCtx;

        // must reply empty should reply with an empty packet!
        let parsed = parser.parse_packet(ctx.clone());

        assert_eq!(parsed, Parsed::new(
            Some(Commands::Acknowledge(Acknowledge::new(ctx.clone()))),
            Some(Commands::NotImplemented(NotImplemented::new(ctx)))));
    }
}
