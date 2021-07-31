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
pub struct ReasonCommand<'a> {
    state: ResponseWriter<'a>
}

impl<'a> ReasonCommand<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseWriter::new(&[])
        }
    }
}

impl Command for ReasonCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        self.state.write_all(stream, ctx.reason())?;
        self.state.end(stream)
    }
}

/**
 * g
 */
#[derive(Debug, PartialEq)]
pub struct ReadRegistersCommand<'a> {
    state: ResponseWriter<'a>
}

impl<'a> ReadRegistersCommand<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseWriter::new(&[])
        }
    }
}

impl Command for ReadRegistersCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        self.state.write_all(stream, ctx.rd_registers())?;
        self.state.end(stream)
    }
}
