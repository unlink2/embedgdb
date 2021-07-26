use super::error::Errors;

/// This function is called whenever the provided memory buffer
/// is not sufficient
/// it allows the handling of the memory buffer;
/// return true if the buffer has been handeled (e.g. transmitted)
/// or false to abort and attempt with a larger buffer
/// T is used to provide a custom context for handling the data
/// this can be nearly any object
pub type OnMemoryFilled<T> = fn(response_data: &[u8], ctx: &mut T) -> bool;

// all supported commands in
// this stub
pub enum Commands<'a, T> {
    Unsupported,
    Retransmit(Retransmit<'a, T>),
    Acknowledge(Acknowledge<'a, T>)
}


/// tracks the current state
/// of response writing
pub struct ResponseState<'a, T> {
    pub fields: &'a [u8],
    pub current_write: usize,
    on_mem_filled: OnMemoryFilled<T>,
    ctx: T
}

impl<'a, T> ResponseState<'a, T> {
    pub fn new(fields: &'a [u8], on_mem_filled: OnMemoryFilled<T>, ctx: T) -> Self {
        Self {
            fields,
            current_write: 0,
            on_mem_filled,
            ctx
        }
    }

    pub fn reset_write(&mut self) {
        self.current_write = 0;
    }

    pub fn write(&mut self, response_data: &mut [u8], byte: u8) -> Result<usize, Errors> {
        if response_data.len() < self.current_write+1 {
            // attempt to handle memory fill
            if !(self.on_mem_filled)(response_data, &mut self.ctx) {
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
pub trait Command<T> {
    /// generates a response for the current command
    /// Returns either an error, or the total amount of bytes written
    /// to the buffer
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors>;
}

pub struct Retransmit<'a, T> {
    state: ResponseState<'a, T>
}

impl<T> Retransmit<'_, T> {
    pub fn new(on_mem_filled: OnMemoryFilled<T>, ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], on_mem_filled, ctx),
        }
    }
}

impl<T> Command<T> for Retransmit<'_, T> {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'-')
    }
}

pub struct Acknowledge<'a, T> {
    state: ResponseState<'a, T>
}

impl<T> Acknowledge<'_, T> {
    pub fn new(on_mem_filled: OnMemoryFilled<T>, ctx: T) -> Self {
        Self {
            state: ResponseState::new(&[], on_mem_filled, ctx)
        }
    }
}

impl<T> Command<T> for Acknowledge<'_, T> {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.write(response_data, b'-')
    }
}
