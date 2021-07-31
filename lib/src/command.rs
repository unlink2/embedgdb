use super::error::Errors;
use super::target::Target;
use super::parser::{Parsed, Parser};
use super::basic::required::*;
use super::stream::{BufferedStream, CommandStream};

// This trait builds a command based on the parer's output
// this allows each target platform to specify exactly which commands
// are supported.
// should return not implemented command by default!
// There will be a few sample implementations of this function
pub trait SupportedCommands<'a, T>
where T: Target {
    fn commands(&self, ctx: &'a T, name: &'a [u8], args: Option<&'a [u8]>) -> Parsed<'a, T> {
        match name {
            b"?" => {
                Parsed::ack(
                    Some(Commands::Reason(ReasonCommand::new(ctx))), ctx)
            },
            _ =>
                Parsed::ack(
                    Some(Commands::NotImplemented(NotImplemented::new())), ctx)
        }
    }
}

// all supported commands in
// this stub
#[derive(Debug, PartialEq)]
pub enum Commands<'a, T>
where T: Target {
    NoCommand,
    Unsupported,
    RetransmitLast, // this is returned if the packet received is a -
    AcknowledgeLast, // this is returned if the packet received a +
    NotImplemented(NotImplemented<'a>),
    Retransmit(Retransmit<'a>),
    Acknowledge(Acknowledge<'a>),
    Reason(ReasonCommand<'a, T>)
}

impl<T> Command for Commands<'_, T>
where T: Target {
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        match self {
            Self::NoCommand
                | Self::Unsupported
                | Self::RetransmitLast
                | Self::AcknowledgeLast => Ok(0),
            Self::NotImplemented(c) => c.response(stream),
            Self::Retransmit(c) => c.response(stream),
            Self::Acknowledge(c) => c.response(stream),
            Self::Reason(c) => c.response(stream)
        }
    }
}

// general interface all commands
// should implement
pub trait Command {
    /// generates a response for the current command
    /// Returns either an error, or the total amount of bytes written
    /// to the buffer
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors>;
}

/// tracks the current state
/// of response writing
#[derive(Debug, PartialEq)]
pub struct ResponseState<'a> {
    pub fields: &'a [u8],
}

impl<'a> ResponseState<'a> {
    pub fn new(fields: &'a [u8]) -> Self {
        Self {
            fields,
        }
    }

    // starts a packet
    pub fn start(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        self.write_force(stream, b'$')
    }

    // ends a packet
    pub fn end(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        let mut size = self.write_force(stream, b'#')?;

        // write checksum byte
        let chksm = (stream.chksm() % 256) as u8;

        size += self.write_hex(stream, chksm)?;

        Ok(size)
    }

    pub fn write_hex(&mut self, stream: &mut dyn CommandStream, byte: u8) -> Result<usize, Errors> {
        let byte = Parser::to_hex_tuple(byte);
        let mut size = self.write_force(stream, byte.0)?;
        size += self.write_force(stream, byte.1)?;
        Ok(size)
    }

    pub fn ok(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        self.write_all(stream, b"OK")
    }

    pub fn error(&mut self, stream: &mut dyn CommandStream, _error: Errors) -> Result<usize, Errors> {
        // TODO write error code
        self.write_all(stream, b"E00")
    }

    pub fn escape(byte: u8) -> u8 {
        byte ^ 0x20
    }

    pub fn write_escape(&mut self, stream: &mut dyn CommandStream, byte: u8) -> Result<usize, Errors> {
        let mut size = self.write_force(stream, b'}')?;
        size += self.write(stream, Self::escape(byte))?;

        Ok(size)
    }

    pub fn write_all(&mut self, stream: &mut dyn CommandStream, bytes: &[u8]) -> Result<usize, Errors> {
        let mut size = 0;
        for byte in bytes {
            size += self.write(stream, *byte)?;
        }
        Ok(size)
    }

    /// forces the write of a byte even if it would normally be escaped
    pub fn write_force(&mut self, stream: &mut dyn CommandStream, byte: u8) -> Result<usize, Errors> {
        stream.write(byte)
    }

    pub fn write(&mut self, stream: &mut dyn CommandStream, byte: u8) -> Result<usize, Errors> {
        match byte {
            // those bytes must always be escaped!
            b'}' | b'$' | b'#' | b'*' => {
                self.write_escape(stream, byte)
            },
            _ => {
                self.write_force(stream, byte)
            }
        }
    }
}

/**
 * Retransmit
 */

#[derive(Debug, PartialEq)]
pub struct Retransmit<'a> {
    state: ResponseState<'a>,
    error: Errors
}

impl<'a> Retransmit<'a> {
    pub fn new(error: Errors) -> Self {
        Self {
            state: ResponseState::new(&[]),
            error
        }
    }
}

impl Command for Retransmit<'_> {
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        stream.reset();
        self.state.write(stream, b'-')
    }
}

/**
 * Acknowledge
 */

#[derive(Debug, PartialEq)]
pub struct Acknowledge<'a> {
    state: ResponseState<'a>
}

impl<'a> Acknowledge<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseState::new(&[])
        }
    }
}

impl Command for Acknowledge<'_> {
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        stream.reset();
        self.state.write(stream, b'+')
    }
}

/**
 * Not implemented
 */

#[derive(Debug, PartialEq)]
pub struct NotImplemented<'a> {
    state: ResponseState<'a>
}

impl<'a> NotImplemented<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseState::new(&[])
        }
    }
}

impl Command for NotImplemented<'_> {
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
        stream.reset();
        let mut size = self.state.start(stream)?;
        size += self.state.end(stream)?;
        Ok(size)
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
    fn it_should_write_data() {
        let mut cmd = Commands::<TestCtx>::NotImplemented(NotImplemented::new());
        let mut stream = BufferedStream::new();

        let size = cmd.response(&mut stream).unwrap();

        assert_eq!(size, 4);
        assert_eq!(stream.pos(), 4);
        assert_eq!(&stream.buffer[..4], b"$#00");
    }

    #[test]
    fn it_should_escape_data() {
        let mut stream = BufferedStream::new();
        let mut state = ResponseState::new(&[]);

        let mut size = state.write(&mut stream, b'$').unwrap();
        size += state.write(&mut stream, b'B').unwrap();

        assert_eq!(size, 3);
        assert_eq!(stream.pos(), 3);
        assert_eq!(&stream.buffer[..4], &[b'}', 4, b'B', 0]);
    }

    #[test]
    fn it_should_fail_if_resize_is_not_possible() {
        let mut stream = BufferedStream::new();
        let mut state = ResponseState::new(&[]);

        // for some reason for loop did not work here
        let mut i = 0;
        while i < stream.len() {
            state.write(&mut stream, b'f').unwrap();
            i = i + 1;
        }

        assert_eq!(stream.len(), 256);
        assert_eq!(stream.pos(), 256);
        let err = state.write_all(&mut stream, b"Hello").unwrap_err();
        assert_eq!(err, Errors::MemoryFilledInterupt);
    }
}
