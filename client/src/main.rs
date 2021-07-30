extern crate embedgdb;
use std::{io::{Read, Write}, net::{TcpListener, TcpStream}};
use embedgdb::{command::Command, parser::Parser};
use embedgdb::target::Target;


#[derive(Debug, Clone, PartialEq)]
struct DebugCtx;
impl Target for DebugCtx {
    fn buffer_full(&mut self, response_data: &[u8]) -> bool {
        false
    }
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0xFF; 1024];

    'readloop: loop {
        buffer.fill(0);
        match stream.read(&mut buffer) {
            Ok(0) => break 'readloop,
            Ok(n) => {
                println!("{} bytes >> {}", n, std::str::from_utf8(&buffer).unwrap_or(""));
                let mut parser = Parser::new(&buffer);

                let result = parser.parse_packet(DebugCtx);

                if let Some(mut response) = result.response {
                    let mut resbuffer = [0xFF; 1024];
                    let size = response.response(&mut resbuffer).unwrap_or(0);

                    println!("{} {:?} res >> {}", size, response, std::str::from_utf8(&resbuffer).unwrap_or(""));
                    if size > 0 {
                        stream.write(&resbuffer)?;
                    }
                }

                if let Some(mut command) = result.command {
                    let mut resbuffer = [0; 1024];
                    let size = command.response(&mut resbuffer).unwrap_or(0);

                    println!("{} {:?} cmd >> {}", size, command, std::str::from_utf8(&resbuffer).unwrap_or(""));
                    if size > 0 {
                        stream.write(&resbuffer)?;
                    }
                }
            },
            Err(err) => return Err(err)
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    // very simple tcp client
    let listener = TcpListener::bind("127.0.0.1:9001")?;

    for stream in listener.incoming() {
        handle(stream?)?;
    }
    Ok(())
}
