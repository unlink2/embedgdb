use super::error::Errors;
use super::target::Target;
use super::parser::{Parsed, Parser};
use super::basic::required::*;

// This trait builds a command based on the parer's output
// this allows each target platform to specify exactly which commands
// are supported.
// should return not implemented command by default!
// There will be a few sample implementations of this function
pub trait SupportedCommands<'a, T>
where T: Target {
    fn commands(&self, ctx: T, name: &'a [u8], args: Option<&'a [u8]>) -> Parsed<T> {
        match name {
            _ =>
                Parsed::new(
                    Some(Commands::Acknowledge(Acknowledge::new(ctx.clone()))),
                    Some(Commands::NotImplemented(NotImplemented::new(ctx))))
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
    NotImplemented(NotImplemented<'a, T>),
    Retransmit(Retransmit<'a, T>),
    Acknowledge(Acknowledge<'a, T>),
}

impl<T> Command<T> for Commands<'_, T>
where T: Target {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        match self {
            Self::NoCommand
                | Self::Unsupported
                | Self::RetransmitLast
                | Self::AcknowledgeLast => Ok(0),
            Self::NotImplemented(c) => c.response(response_data),
            Self::Retransmit(c) => c.response(response_data),
            Self::Acknowledge(c) => c.response(response_data)
        }
    }
}

// general interface all commands
// should implement
pub trait Command<T>
where T: Target {
    /// generates a response for the current command
    /// Returns either an error, or the total amount of bytes written
    /// to the buffer
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors>;
}

/// tracks the current state
/// of response writing
#[derive(Debug, PartialEq)]
pub struct ResponseState<'a, T>
where T: Target {
    pub fields: &'a [u8],
    pub current_write: usize,
    pub chksm: u32, // buffer for checksum
    ctx: T
}

impl<'a, T> ResponseState<'a, T>
where T: Target {
    pub fn new(fields: &'a [u8], ctx: T) -> Self {
        Self {
            fields,
            current_write: 0,
            chksm: 0,
            ctx
        }
    }

    pub fn reset_write(&mut self) {
        self.current_write = 0;
        self.chksm = 0;
    }


    // starts a packet
    pub fn start(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.write_force(response_data, b'$')
    }

    // ends a packet
    pub fn end(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        let mut size = self.write_force(response_data, b'#')?;

        self.chksm += Parser::add_chksm(response_data);
        // write checksum byte
        let chksum = Parser::to_hex_tuple((self.chksm % 256) as u8);

        size += self.write_force(response_data, chksum.0)?;
        size += self.write_force(response_data, chksum.1)?;
        Ok(size)
    }

    pub fn ok(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.write_all(response_data, b"OK")
    }

    pub fn error(&mut self, response_data: &mut [u8], _error: Errors) -> Result<usize, Errors> {
        // TODO write error code
        self.write_all(response_data, b"E 00")
    }

    pub fn escape(byte: u8) -> u8 {
        byte ^ 0x20
    }

    pub fn write_escape(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        let mut size = self.write_force(response_data, b'}')?;
        size += self.write(response_data, Self::escape(byte))?;

        Ok(size)
    }

    pub fn write_all(&mut self, response_data: &mut [u8], bytes: &[u8]) -> Result<usize, Errors> {
        let mut size = 0;
        for byte in bytes {
            size += self.write(response_data, *byte)?;
        }
        Ok(size)
    }

    /// forces the write of a byte even if it would normally be escaped
    pub fn write_force(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        let start = self.current_write;
        if response_data.len() < self.current_write+1 {
            // do not clear before adding bytes to checksum,
            self.chksm += Parser::add_chksm(response_data);

            // attempt to handle memory fill
            if !self.ctx.buffer_full(response_data) {
                return Err(Errors::MemoryFilledInterupt);
            } else {
                self.current_write = 0;
            }
        }
        response_data[self.current_write] = byte;
        self.current_write += 1;
        Ok(self.current_write-start)
    }

    pub fn write(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        match byte {
            // those bytes must always be escaped!
            b'}' | b'$' | b'#' | b'*' => {
                self.write_escape(response_data, byte)
            },
            _ => {
                self.write_force(response_data, byte)
            }
        }
    }
}

/**
 * Retransmit
 */

#[derive(Debug, PartialEq)]
pub struct Retransmit<'a, T>
where T: Target {
    state: ResponseState<'a, T>,
    error: Errors
}

impl<T> Retransmit<'_, T>
where T: Target {
    pub fn new(ctx: T, error: Errors) -> Self {
        Self {
            state: ResponseState::new(&[], ctx),
            error
        }
    }
}

impl<T> Command<T> for Retransmit<'_, T>
where T: Target {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'-')
    }
}

/**
 * Acknowledge
 */

#[derive(Debug, PartialEq)]
pub struct Acknowledge<'a, T>
where T: Target {
    state: ResponseState<'a, T>
}

impl<T> Acknowledge<'_, T>
where T: Target {
    pub fn new(ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx)
        }
    }
}

impl<T> Command<T> for Acknowledge<'_, T>
where T: Target {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'+')
    }
}

/**
 * Not implemented
 */

#[derive(Debug, PartialEq)]
pub struct NotImplemented<'a, T>
where T: Target {
    state: ResponseState<'a, T>
}

impl<T> NotImplemented<'_, T>
where T: Target {
    pub fn new(ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx)
        }
    }
}

impl<T> Command<T> for NotImplemented<'_, T>
where T: Target {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        let mut size = self.state.start(response_data)?;
        size += self.state.end(response_data)?;
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
        fn buffer_full(&mut self, response_data: &[u8]) -> bool {
            false
        }
    }

    #[test]
    fn it_should_write_data() {
        let mut cmd = Commands::NotImplemented(NotImplemented::new(TestCtx));
        let mut buffer = [0; 4];

        let size = cmd.response(&mut buffer).unwrap();

        assert_eq!(size, 4);
        assert_eq!(&buffer, b"$#00");
    }

    #[test]
    fn it_should_escape_data() {
        let mut buffer = [0; 4];
        let mut state = ResponseState::new(&[], TestCtx);

        let mut size = state.write(&mut buffer, b'$').unwrap();
        size += state.write(&mut buffer, b'B').unwrap();

        assert_eq!(size, 3);
        assert_eq!(&buffer, &[b'}', 4, b'B', 0]);
    }

    #[test]
    fn it_should_fail_if_resize_is_not_possible() {
        let mut buffer = [0; 4];
        let mut state = ResponseState::new(&[], TestCtx);

        let err = state.write_all(&mut buffer, b"Hello").unwrap_err();
        assert_eq!(err, Errors::MemoryFilledInterupt);
    }
}
