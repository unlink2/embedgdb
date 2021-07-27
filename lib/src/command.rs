use super::error::Errors;
use super::target::Target;


// all supported commands in
// this stub
pub enum Commands<'a, T>
where T: Target {
    Unsupported,
    Retransmit(Retransmit<'a, T>),
    Acknowledge(Acknowledge<'a, T>),
}


/// tracks the current state
/// of response writing
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

    // adds a buffe rto chksm
    fn add_chksm(&mut self, response_data: &mut [u8]) {
        // never include $ and # in checksum. -> this is fine because they always need
        // to be escaped anyway so in a well-formed packet they should always appear at the
        // start/end
        for b in response_data {
            match *b {
                b'$' | b'#' => (),
                _ => self.chksm += *b as u32
            }
        }
    }

    // starts a packet
    pub fn start(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.write(response_data, b'$')
    }

    // ends a packet
    pub fn end(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.write(response_data, b'#')?;

        self.add_chksm(response_data);
        // write checksum byte
        let chksum = (self.chksm % 256) as u8;

        // TODO hex digit converter!
        self.write(response_data, (chksum & 0xF0) >> 4)?;
        self.write(response_data, chksum & 0x0F)
    }

    pub fn escape(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        todo!()
    }

    pub fn write(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        if response_data.len() < self.current_write+1 {
            // do not clear before adding bytes to checksum,
            self.add_chksm(response_data);

            // attempt to handle memory fill
            if !self.ctx.on_mem_filled(response_data) {
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

// general interface all commands
// should implement
pub trait Command<T>
where T: Target {
    /// generates a response for the current command
    /// Returns either an error, or the total amount of bytes written
    /// to the buffer
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors>;
}

pub struct Retransmit<'a, T>
where T: Target {
    state: ResponseState<'a, T>
}

impl<T> Retransmit<'_, T>
where T: Target {
    pub fn new(ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx),
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
        self.state.write(response_data, b'-')
    }
}
