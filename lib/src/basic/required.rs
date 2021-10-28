/*
 * All the required commands
 */

use crate::command::*;
use crate::error::Errors;
use crate::parser::Parser;
use crate::stream::Stream;
use crate::target::Target;

/**
 * ?
 */

#[derive(Debug, PartialEq)]
pub struct ReasonCommand<'a> {
    state: ResponseWriter<'a>,
}

impl<'a> ReasonCommand<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseWriter::new(&[]),
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
    state: ResponseWriter<'a>,
}

impl<'a> Default for ReadRegistersCommand<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ReadRegistersCommand<'a> {
    pub fn new() -> Self {
        Self {
            state: ResponseWriter::new(&[]),
        }
    }
}

impl Command for ReadRegistersCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        ctx.rd_registers(stream)?;
        self.state.end(stream)?;
        Ok(stream.pos())
    }
}

/**
 * G
 */

#[derive(Debug, PartialEq)]
pub struct WriteRegistersCommand<'a> {
    state: ResponseWriter<'a>,
}

impl<'a> Default for ReasonCommand<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> WriteRegistersCommand<'a> {
    pub fn new(args: &'a [u8]) -> Self {
        Self {
            state: ResponseWriter::new(args),
        }
    }
}

impl Command for WriteRegistersCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;
        match ctx.wr_registers(self.state.fields) {
            Ok(_) => self.state.ok(stream)?,
            Err(err) => self.state.error(stream, err)?,
        };
        self.state.end(stream)?;
        Ok(stream.pos())
    }
}

/**
 * m
 */
#[derive(Debug, PartialEq)]
pub struct ReadMemoryCommand<'a> {
    state: ResponseWriter<'a>,
}

impl<'a> ReadMemoryCommand<'a> {
    pub fn new(args: &'a [u8]) -> Self {
        Self {
            state: ResponseWriter::new(args),
        }
    }
}

impl Command for ReadMemoryCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        // expecting 2 tokens, addr and size
        let mut parser = Parser::new(self.state.fields);
        let addr = parser.next_token();
        let size = parser.next_token();

        if let (Some(addr), Some(size)) = (addr, size) {
            let addr = Parser::from_hexu(addr);
            let size = Parser::from_hexu(size);

            if let (Some(addr), Some(size)) = (addr, size) {
                ctx.rd_memory(addr as *const u8, size as usize, stream)?;
                self.state.end(stream)?;
                Ok(stream.pos())
            } else {
                Err(Errors::BadNumber)
            }
        } else {
            Err(Errors::InsufficientArguments)
        }
    }
}

/**
 * M
 */
#[derive(Debug, PartialEq)]
pub struct WriteMemoryCommand<'a> {
    state: ResponseWriter<'a>,
}

impl<'a> WriteMemoryCommand<'a> {
    pub fn new(args: &'a [u8]) -> Self {
        Self {
            state: ResponseWriter::new(args),
        }
    }
}

impl Command for WriteMemoryCommand<'_> {
    fn response(&mut self, stream: &mut dyn Stream, ctx: &mut dyn Target) -> Result<usize, Errors> {
        stream.reset();
        self.state.start(stream)?;

        // expecting 2 tokens, addr and size
        let mut parser = Parser::new(self.state.fields);
        let addr = parser.next_token();
        let size = parser.next_token();
        let bytes = parser.next_token();

        if let (Some(addr), Some(size), Some(bytes)) = (addr, size, bytes) {
            let addr = Parser::from_hexu(addr);
            let size = Parser::from_hexu(size);

            // mismatched lenght!
            if let (Some(addr), Some(size)) = (addr, size) {
                if bytes.len() / 2 != size {
                    Err(Errors::LengthMismatch)
                } else {
                    ctx.wr_memory(addr as *const u8, bytes)?;
                    self.state.ok(stream)?;
                    self.state.end(stream)?;
                    Ok(stream.pos())
                }
            } else {
                Err(Errors::BadNumber)
            }
        } else {
            Err(Errors::InsufficientArguments)
        }
    }
}
