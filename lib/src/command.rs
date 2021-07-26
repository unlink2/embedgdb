use super::error::Errors;
use super::arch::Arch;


// all supported commands in
// this stub
pub enum Commands<'a, T>
where T: Arch {
    Unsupported,
    Retransmit(Retransmit<'a, T>),
    Acknowledge(Acknowledge<'a, T>)
}


/// tracks the current state
/// of response writing
pub struct ResponseState<'a, T>
where T: Arch {
    pub fields: &'a [u8],
    pub current_write: usize,
    ctx: T
}

impl<'a, T> ResponseState<'a, T>
where T: Arch {
    pub fn new(fields: &'a [u8], ctx: T) -> Self {
        Self {
            fields,
            current_write: 0,
            ctx
        }
    }

    pub fn reset_write(&mut self) {
        self.current_write = 0;
    }

    pub fn write(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        if response_data.len() < self.current_write+1 {
            // attempt to handle memory fill
            if !self.ctx.on_mem_filled(response_data) {
                return Err(Errors::MemoryFilledInterupt);
            }
        }
        response_data[0] = byte;
        self.current_write += 1;
        Ok(self.current_write)
    }
}

// general interface all commands
// should implement
pub trait Command<T>
where T: Arch {
    /// generates a response for the current command
    /// Returns either an error, or the total amount of bytes written
    /// to the buffer
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors>;
}

pub struct Retransmit<'a, T>
where T: Arch {
    state: ResponseState<'a, T>
}

impl<T> Retransmit<'_, T>
where T: Arch {
    pub fn new(ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx),
        }
    }
}

impl<T> Command<T> for Retransmit<'_, T>
where T: Arch {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'-')
    }
}

pub struct Acknowledge<'a, T>
where T: Arch {
    state: ResponseState<'a, T>
}

impl<T> Acknowledge<'_, T>
where T: Arch {
    pub fn new(ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx)
        }
    }
}

impl<T> Command<T> for Acknowledge<'_, T>
where T: Arch {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'-')
    }
}
