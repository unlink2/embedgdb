use super::command::*;
use super::error::Errors;
use super::stream::Stream;

// Holds the Acknowledge Packet and command packet

#[derive(Debug, PartialEq)]
pub struct Parsed<'a> {
    pub response: Option<Commands<'a>>,
    pub command: Option<Commands<'a>>,
}

impl<'a> Parsed<'a> {
    pub fn new(response: Option<Commands<'a>>, command: Option<Commands<'a>>) -> Self {
        Self { response, command }
    }

    pub fn ack(command: Option<Commands<'a>>) -> Self {
        Self::new(Some(Commands::Acknowledge(Acknowledge::new())), command)
    }
}

#[derive(Copy, Clone)]
pub enum Endianness {
    Big,
    Little,
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
        Self { packet, current: 0 }
    }

    fn retransmit(error: Errors) -> Parsed<'a> {
        Parsed::new(Some(Commands::Retransmit(Retransmit::new(error))), None)
    }

    // packet layout:
    // $<optional id:>packet-data#checksum
    // if this function causes an error
    // a retransmit packet should be sent
    pub fn parse_packet(&mut self, cmds: &'a dyn SupportedCommands<'a>) -> Parsed<'a> {
        // there are 2 special cases where there is no checksum
        if self.is_match(b'-') {
            return Parsed::new(Some(Commands::RetransmitLast), None);
        } else if self.is_match(b'+') {
            return Parsed::new(Some(Commands::AcknowledgeLast), None);
        }

        // first char needs to be $
        if !self.is_match(b'$') {
            // bail
            return Self::retransmit(Errors::UnexpectedIntroduction);
        }

        // read packet name
        // packet names either are terminated by #, space, comma or semicolon
        let name = self.parse_name();

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
            return Self::retransmit(Errors::NotTerminated);
        }
        // is checksum ok?
        if !self.verify_chksm() {
            return Self::retransmit(Errors::InvalidChecksum);
        }
        // get to end
        self.advance();

        cmds.commands(name, args)
    }

    // parses a single token
    pub fn next_token(&mut self) -> Option<&'a [u8]> {
        if !self.is_at_end() {
            let token = self.parse_token();
            self.advance();
            Some(token)
        } else {
            None
        }
    }

    pub fn parse_name(&mut self) -> &'a [u8] {
        match self.peek() {
            b'v' | b'q' => self.parse_token(),
            _ => &self.packet[self.current..self.current + 1],
        }
    }

    /// parses a single token
    /// and returns a slice containing it
    pub fn parse_token(&mut self) -> &'a [u8] {
        let start = self.current;
        while !self.is_at_end() && !self.is_term() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    // parse all remaining tokens into a single slice
    // because we do not have dynamic memory tokens have to be read
    // parsed in a later step
    pub fn parse_until_end(&mut self) -> &'a [u8] {
        let start = self.current;
        while self.peek() != b'#' && !self.is_at_end() {
            self.advance();
        }
        &self.packet[start..self.current]
    }

    pub fn is_term(&self) -> bool {
        matches!(self.peek(), b' ' | b',' | b'#' | b';' | b':')
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.packet.len()
    }

    pub fn advance(&mut self) -> u8 {
        self.current += 1;
        *self.packet.get(self.current).unwrap_or(&b'\0')
    }

    pub fn peek(&self) -> u8 {
        *self.packet.get(self.current).unwrap_or(&b'\0')
    }

    pub fn next(&self) -> u8 {
        *self.packet.get(self.current + 1).unwrap_or(&b'\0')
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
                _ => sum += *b as u32,
            }
        }
        sum
    }

    // adds a buffer to chksm
    pub fn chksm(response_data: &[u8]) -> u32 {
        Self::add_chksm(response_data) % 256
    }

    pub fn is_digit(b: u8) -> bool {
        (b'0'..=b'9').contains(&b)
    }

    pub fn is_hex(b: u8) -> bool {
        (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
    }

    pub fn is_hex_digit(b: u8) -> bool {
        Self::is_hex(b) || Self::is_digit(b)
    }

    pub fn from_hex(b: u8) -> Option<u8> {
        if (b'0'..=b'9').contains(&b) {
            Some(b - b'0')
        } else if (b'A'..=b'F').contains(&b) {
            Some(b - b'A' + 10)
        } else if (b'a'..=b'f').contains(&b) {
            Some(b - b'a' + 10)
        } else {
            None
        }
    }

    /// non-size-bounded hex conversion
    /// for when the size does not need to fit a particular
    /// bound
    pub fn from_hexu(b: &[u8]) -> Option<usize> {
        // filter for the first \0 if it exists
        let end_index = b.iter().position(|&v| v == 0).unwrap_or(b.len());
        let b = &b[0..end_index];

        let mut result = 0;
        let shift_len = (b.len() - 1) * 4;
        for (i, byte) in b.iter().enumerate() {
            let val = Self::from_hex(*byte)? as usize;

            result |= val << (shift_len - 4 * i);
        }
        Some(result)
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

    // converts a stream of bytes to hex and write it to
    // the output stream
    pub fn to_hex8(b: u8, stream: &mut dyn Stream) -> Result<(), Errors> {
        let t = Self::to_hex_tuple(b);
        stream.write(t.0)?;
        stream.write(t.1)?;
        Ok(())
    }

    pub fn to_hexu(b: &[u8], stream: &mut dyn Stream) -> Result<(), Errors> {
        for byte in b {
            Self::to_hex8(*byte, stream)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic::required::*;
    use crate::stream::BufferedStream;

    struct TestCommands;
    impl<'a> SupportedCommands<'a> for TestCommands {}

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
    fn it_should_write_hex8() {
        let mut s = BufferedStream::new();
        Parser::to_hex8(0xAF, &mut s).unwrap();
        assert_eq!(&s.buffer[..2], b"af");
    }

    #[test]
    fn it_should_write_hex16() {
        let mut s = BufferedStream::new();
        Parser::to_hexu(&[0x12, 0xAF], &mut s).unwrap();
        assert_eq!(&s.buffer[..4], b"12af");
    }

    #[test]
    fn it_should_write_hex32() {
        let mut s = BufferedStream::new();
        Parser::to_hexu(&[0xee, 0xdd, 0x12, 0xAF], &mut s).unwrap();
        assert_eq!(&s.buffer[..8], b"eedd12af");
    }

    #[test]
    fn it_should_read_hex8() {
        assert_eq!(Parser::from_hexu(&[b'A', b'B']).unwrap(), 0xAB);
    }

    #[test]
    fn it_should_read_hex8_with_padding() {
        assert_eq!(Parser::from_hexu(&[b'A', b'B', 0, 0, 0]).unwrap(), 0xAB);
    }

    #[test]
    fn it_should_read_hex16() {
        assert_eq!(
            Parser::from_hexu(&[b'A', b'B', b'c', b'd']).unwrap(),
            0xABcd
        );
    }

    #[test]
    fn it_should_read_hex32() {
        assert_eq!(
            Parser::from_hexu(&[b'A', b'B', b'c', b'd', b'1', b'2', b'3', b'4']).unwrap(),
            0xABcd1234
        );
    }

    #[test]
    fn it_should_read_hex32_with_padding() {
        assert_eq!(
            Parser::from_hexu(&[b'A', b'B', b'c', b'd', b'1', b'2', b'3', 0, 0, 0, 0, 0]).unwrap(),
            0xABcd123
        );
    }

    #[test]
    fn it_should_read_hex_be() {
        let mut s = BufferedStream::new();
        let be = 0xBFC00000 as u32;
        let mut be_bytes = be.to_be_bytes();
        be_bytes.reverse();
        Parser::to_hexu(&be_bytes, &mut s).unwrap();

        assert_eq!(s.buffer[0..8], b"0000c0bf"[..]);
        assert_eq!(
            (Parser::from_hexu(&s.buffer[0..8]).unwrap() as u32).to_le_bytes(),
            be.to_be_bytes()[..]
        );
    }

    #[test]
    fn it_should_calculate_checksums() {
        assert_eq!(Parser::chksm(b"$vMustReplyEmpty#3a"), 0x3a);
    }

    #[test]
    fn it_should_parse_packet() {
        let chksm = "$vMustReplyEmpty#3a".as_bytes();

        let mut parser = Parser::new(chksm);

        // must reply empty should reply with an empty packet!
        let parsed = parser.parse_packet(&TestCommands);

        assert_eq!(
            parsed,
            Parsed::new(
                Some(Commands::Acknowledge(Acknowledge::new())),
                Some(Commands::NotImplemented(NotImplemented::new()))
            )
        );
    }

    #[test]
    fn it_should_parse_to_end() {
        let chksm = "$vMustReplyEmpty#3a".as_bytes();

        let mut parser = Parser::new(chksm);

        // must reply empty should reply with an empty packet!
        let _ = parser.parse_packet(&TestCommands);

        assert!(parser.is_at_end());
    }

    #[test]
    fn it_should_read_name() {
        let chksm = "$g#67".as_bytes();

        let mut parser = Parser::new(chksm);

        // must reply empty should reply with an empty packet!
        let parsed = parser.parse_packet(&TestCommands);

        assert_eq!(
            parsed,
            Parsed::new(
                Some(Commands::Acknowledge(Acknowledge::new())),
                Some(Commands::ReadRegister(ReadRegistersCommand::new()))
            )
        );
    }

    #[test]
    fn it_should_read_name_long() {
        let chksm = "$G64#b1".as_bytes();

        let mut parser = Parser::new(chksm);

        // must reply empty should reply with an empty packet!
        let parsed = parser.parse_packet(&TestCommands);

        assert_eq!(
            parsed,
            Parsed::new(
                Some(Commands::Acknowledge(Acknowledge::new())),
                Some(Commands::WriteRegister(WriteRegistersCommand::new(b"64")))
            )
        );
    }

    #[test]
    fn it_should_parse_long_packet() {
        let packet = "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;no-resumed+;xmlRegisters=i386#6a".as_bytes();
        let mut parser = Parser::new(packet);

        let parsed = parser.parse_packet(&TestCommands);

        assert_eq!(
            parsed,
            Parsed::new(
                Some(Commands::Acknowledge(Acknowledge::new())),
                Some(Commands::NotImplemented(NotImplemented::new()))
            )
        );
    }

    #[test]
    fn it_should_parse_to_end_long_packet() {
        let packet = "$qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-events+;vfork-events+;exec-events+;vContSupported+;QThreadEvents+;no-resumed+;xmlRegisters=i386#6a".as_bytes();
        let mut parser = Parser::new(packet);

        let _ = parser.parse_packet(&TestCommands);
        assert!(parser.is_at_end());
    }

    #[test]
    fn it_should_parse_tokens() {
        let packet = b"token1;token2,token3";

        let mut parser = Parser::new(packet);

        let t1 = parser.next_token();
        assert_eq!(t1, Some(&b"token1"[..]));
        assert!(!parser.is_at_end());

        let t2 = parser.next_token();
        assert!(!parser.is_at_end());
        assert_eq!(t2, Some(&b"token2"[..]));

        let t3 = parser.next_token();
        assert!(parser.is_at_end());
        assert_eq!(t3, Some(&b"token3"[..]));

        let t4 = parser.next_token();
        assert!(parser.is_at_end());
        assert_eq!(t4, None);
    }
}
