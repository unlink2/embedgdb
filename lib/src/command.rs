use super::error::Errors;
use super::target::Target;
use super::parser::Parser;


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
        self.write(response_data, b'$')
    }

    // ends a packet
    pub fn end(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.write(response_data, b'#')?;

        self.chksm += Parser::add_chksm(response_data);
        // write checksum byte
        let chksum = Parser::to_hex_tuple((self.chksm % 256) as u8);

        self.write(response_data, chksum.0)?;
        self.write(response_data, chksum.1)
    }

    pub fn escape(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        todo!()
    }

    pub fn write(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
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
        Ok(self.current_write)
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
        self.state.start(response_data)?;
        self.state.end(response_data)
    }
}

