/*
 * All the required commands
 */

use crate::command::*;
use crate::parser::{Parser, Parsed};
use crate::target::Target;
use crate::error::Errors;

/**
 * ?
 */

#[derive(Debug, PartialEq)]
pub struct ReasonCommand<'a, T>
where T: Target {
    state: ResponseState<'a, T>
}

impl<'a, T> ReasonCommand<'a, T>
where T: Target {
    pub fn new(ctx: &'a T) -> Self {
        Self {
            state: ResponseState::new(&[], ctx)
        }
    }
}

impl<T> Command<T> for ReasonCommand<'_, T>
where T: Target {
    fn response(&mut self, response_data: &mut [u8]) -> Result<usize, Errors> {
        self.state.reset_write();
        self.state.start(response_data)?;

        let ctx = self.state.ctx;
        self.state.write_all(response_data, ctx.reason())?;
        self.state.end(response_data)
    }
}

