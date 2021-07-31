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
    pub fn new(response: Option<Commands<'a, T>>, command: Option<Commands<'a, T>>) -> Self {
        Self {response, command}
    }

    pub fn ack(command: Option<Commands<'a, T>>, ctx: &'a T) -> Self {
        Self::new(
            Some(Commands::Acknowledge(Acknowledge::new())), command)
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
#[derive(Clone)]
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

    fn retransmit<T>(ctx: &'a T, error: Errors) -> Parsed<'a, T>
    where T: Target {
        Parsed::new(
            Some(Commands::Retransmit(Retransmit::new(error))), None)
    }

    // packet layout:
    // $<optional id:>packet-data#checksum
    // if this function causes an error
    // a retransmit packet should be sent
    pub fn parse_packet<T>(&mut self, ctx: &'a T, cmds: &'a dyn SupportedCommands<'a, T>) -> Parsed<'a, T>
    where T: Target {
        // there are 2 special cases where there is no checksum
        if self.is_match(b'-') {
            return Parsed::new(Some(Commands::RetransmitLast), None);
        } else if self.is_match(b'+')  {
            return Parsed::new(Some(Commands::AcknowledgeLast), None);
        }

        // first char needs to be $
        if !self.is_match(b'$') {
            // bail
            return Self::retransmit(ctx, Errors::UnexpectedIntroduction);
        }

        // read packet name
        // packet names either are terminated by #, space, comma or semicolon
        let name = self.parse_token();

        // only if not #
        let args = if self.peek() != b'#' {
            self.advance(); // must be other terminator, skip it
            // read rest of data, those will be parsed when the packet is interpreted/executed
            Some(self.parse_until_end())
        } else {
            None
        };

        // read end-delim
        if !self.is_match(b'#') {
            // retransmit - the packet never terminated!
            return Self::retransmit(ctx, Errors::NotTerminated);
        }
        // is checksum ok?
        if !self.verify_chksm() {
            return Self::retransmit(ctx, Errors::InvalidChecksum);
        }

        cmds.commands(ctx, name, args)
    }

    /// parses a single token
    /// and returns a slice containing it
    pub fn parse_token(&mut self) -> &'a [u8] {
        let start = self.current;
        while !self.is_at_end()
            && !self.is_term() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    // parse all remaining tokens into a single slice
    // because we do not have dynamic memory tokens have to be read
    // parsed in a later step
    pub fn parse_until_end(&mut self) -> &'a [u8] {
        let start = self.current;
        while self.peek() != b'#'
            && !self.is_at_end() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    pub fn is_term(&self) -> bool {
        match self.peek() {
            b' ' | b',' | b'#' | b';' | b':' => true,
            _ => false,
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.packet.len()-1
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
    pub fn verify_chksm(&mut self) -> bool {
        // read next 2 bytes
        let b0 = Self::from_hex(self.peek());
        let b1 = Self::from_hex(self.advance());

        if let (Some(b0), Some(b1)) = (b0, b1) {
            // now we have a sum, calculate based on data and see!
            let sum = (b0 << 4) | b1;
            let calc = Self::chksm(self.packet) as u8;
            sum == calc
        } else {
            // in all other cases bail with bad checksum!
            false
        }
    }

    fn add_chksm(response_data: &[u8]) -> u32 {
        // never include $ and # in checksum. -> this is fine because they always need
        // to be escaped anyway so in a well-formed packet they should always appear at the
        // start/end
        let mut sum = 0;
        'cloop: for b in response_data {
            match *b {
                b'$' => (),
                b'#' => break 'cloop,
                _ => sum += *b as u32
            }
        }
        return sum;
    }

    // adds a buffer to chksm
    pub fn chksm(response_data: &[u8]) -> u32 {
        Self::add_chksm(response_data) % 256
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
            Some(b - b'A' + 10)
        } else if b >= b'a' && b <= b'f' {
            Some(b - b'a' + 10)
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
            Some(b + b'a' - 10)
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

    struct TestCommands;
    impl<'a, T> SupportedCommands<'a, T> for TestCommands
            where T: Target {}

    #[derive(Debug, Clone, PartialEq)]
    struct TestCtx;
    impl Target for TestCtx {
        fn buffer_full(&self, response_data: &[u8]) -> bool {
            false
        }
    }

    #[test]
    fn it_should_parse_hex() {
        assert_eq!(Parser::to_hex(15), Some(b'f'));
        assert_eq!(Parser::to_hex(6), Some(b'6'));
        assert_eq!(Parser::to_hex(16), None);
    }

    #[test]
    fn it_should_parse_hex_tupel() {
        assert_eq!(Parser::to_hex_tuple(0xA7), (b'a', b'7'));
    }

    #[test]
    fn it_should_calculate_checksums() {
        assert_eq!(Parser::chksm(b"$vMustReplyEmpty#3a"), 0x3a);
    }

    #[test]
    fn it_should_parse_packet() {
        let chksm = "$vMustReplyEmpty#3a".as_bytes();

        let mut parser = Parser::new(chksm);
        let ctx = TestCtx;

        // must reply empty should reply with an empty packet!
        let parsed = parser.parse_packet(&ctx, &TestCommands);

        assert_eq!(parsed, Parsed::new(
            Some(Commands::Acknowledge(Acknowledge::new())),
            Some(Commands::NotImplemented(NotImplemented::new()))));
    }

    #[test]
    fn it_should_parse_to_end() {
        let chksm = "$vMustReplyEmpty#3a".as_bytes();

        let mut parser = Parser::new(chksm);
        let ctx = TestCtx;

        // must reply empty should reply with an empty packet!
        let _ = parser.parse_packet(&ctx, &TestCommands);

        assert!(parser.is_at_end());
    }

    #[test]
    fn it_should_parse_long_packet() {
        let packet = "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;no-resumed+;xmlRegisters=i386#6a".as_bytes();
        let mut parser = Parser::new(packet);
        let ctx = TestCtx;

        let parsed = parser.parse_packet(&ctx, &TestCommands);

        assert_eq!(parsed, Parsed::new(
            Some(Commands::Acknowledge(Acknowledge::new())),
            Some(Commands::NotImplemented(NotImplemented::new()))));
    }

    #[test]
    fn it_should_parse_to_end_long_packet() {
        let packet = "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;no-resumed+;xmlRegisters=i386#6a".as_bytes();
        let mut parser = Parser::new(packet);
        let ctx = TestCtx;

        let _ = parser.parse_packet(&ctx, &TestCommands);
        assert!(parser.is_at_end());
    }
}
