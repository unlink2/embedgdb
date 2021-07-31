/*
 * All the required commands
 */

use crate::command::*;
use crate::parser::{Parser, Parsed};
use crate::target::Target;
use crate::error::Errors;
use crate::stream::CommandStream;

/**
 * ?
 */

#[derive(Debug, PartialEq)]
pub struct ReasonCommand<'a, T>
where T: Target {
    ctx: &'a T,
    state: ResponseState<'a>
}

impl<'a, T> ReasonCommand<'a, T>
where T: Target {
    pub fn new(ctx: &'a T) -> Self {
        Self {
            ctx,
            state: ResponseState::new(&[])
        }
    }
}

impl<T> Command for ReasonCommand<'_, T>
where T: Target {
    fn response(&mut self, stream: &mut dyn CommandStream) -> Result<usize, Errors> {
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
pub struct ReadRegistersCommand;
