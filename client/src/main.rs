extern crate embedgdb;
use std::{io::{Read, Write}, net::{TcpListener, TcpStream}};
use embedgdb::{command::{SupportedCommands, Command}, parser::Parser};
use embedgdb::target::VirtualTarget;
use embedgdb::stream::BufferedStream;

struct DebugCommands;
impl<'a> SupportedCommands<'a> for DebugCommands {
}


fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0xFF; 2048];

    let mut target = VirtualTarget::new();

    'readloop: loop {
        buffer.fill(0);
        match stream.read(&mut buffer) {
            Ok(0) => break 'readloop,
            Ok(n) => {
                println!("{} bytes >> {}", n, std::str::from_utf8(&buffer).unwrap_or(""));
                let mut parser = Parser::new(&buffer);

                let result = parser.parse_packet(&DebugCommands);

                if let Some(mut response) = result.response {
                    let mut rstream = BufferedStream::new();
                    let size = response.response(&mut rstream, &mut target).unwrap();

                    println!("{} {:?} res >> {}", size, response, std::str::from_utf8(&rstream.buffer).unwrap_or(""));
                    if size > 0 {
                        stream.write(&rstream.buffer)?;
                    }
                }

                if let Some(mut command) = result.command {
                    let mut rstream = BufferedStream::new();
                    let size = command.response(&mut rstream, &mut target).unwrap();

                    println!("{} {:?} cmd >> {}", size, command, std::str::from_utf8(&rstream.buffer).unwrap_or(""));
                    if size > 0 {
                        stream.write(&rstream.buffer)?;
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
