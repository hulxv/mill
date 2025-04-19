use mill_io::event_loop::{EventHandler, EventLoop};
use nix::sys::epoll::EpollFlags;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::os::unix::io::{AsRawFd, RawFd};

struct HttpHandler {
    listener: TcpListener,
}

impl EventHandler for HttpHandler {
    fn handle_read(&mut self, _fd: RawFd) -> io::Result<()> {
        match self.listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0; 1024];
                stream.read(&mut buffer)?;

                let response = "HTTP/1.1 200 OK\r\n\
                              Content-Length: 13\r\n\
                              \r\n\
                              Hello, World from My Cool Event-loop library!!";

                stream.write_all(response.as_bytes())?;
                stream.flush()?;
                println!("Response sent to client: {:?}", stream.peer_addr()?);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // No incoming connections right now
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    fn handle_write(&mut self, _fd: RawFd) -> io::Result<()> {
        Ok(()) // Nothing to do for write events
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(true)?;

    let mut event_loop = EventLoop::new()?;
    let handler = Box::new(HttpHandler {
        listener: listener.try_clone()?,
    });

    event_loop.add_handler(
        listener.as_raw_fd(),
        EpollFlags::EPOLLIN | EpollFlags::EPOLLET,
        handler,
    )?;

    println!("Server listening on: {listener:?}");
    event_loop.run()?;
    Ok(())
}
