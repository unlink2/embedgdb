/*
 * All the required commands
 */

use crate::command::*;
use crate::parser::{Parser, Parsed};
use crate::target::Target;
use crate::error::Errors;
use crate::stream::Stream;

/**
 * ?
 */

#[derive(Debug, PartialEq)]
pub struct ReasonCommand<'a, T>
where T: Target {
    ctx: &'a T,
    state: ResponseWriter<'a>
}

impl<'a, T> ReasonCommand<'a, T>
where T: Target {
    pub fn new(ctx: &'a T) -> Self {
        Self {
            ctx,
            state: ResponseWriter::new(&[])
        }
    }
}

impl<T> Command for ReasonCommand<'_, T>
where T: Target {
    fn response(&mut self, stream: &mut dyn Stream) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        let ctx = self.ctx;
        self.state.write_all(stream, ctx.reason())?;
        self.state.end(stream)
    }
}

/**
 * g
 */
#[derive(Debug, PartialEq)]
pub struct ReadRegistersCommand<'a, T>
where T: Target {
    ctx: &'a T,
    state: ResponseWriter<'a>
}

impl<'a, T> ReadRegistersCommand<'a, T>
where T: Target {
    pub fn new(ctx: &'a T) -> Self {
        Self {
            ctx,
            state: ResponseWriter::new(&[])
        }
    }
}

impl<T> Command for ReadRegistersCommand<'_, T>
where T: Target {
    fn response(&mut self, stream: &mut dyn Stream) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        let ctx = self.ctx;
        self.state.write_all(stream, ctx.registers())?;
        self.state.end(stream)
    }
}
