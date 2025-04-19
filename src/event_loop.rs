use nix::sys::epoll::{
    Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollOp, epoll_create1, epoll_ctl, epoll_wait,
};
use std::collections::HashMap;
use std::io::{self, Result};
use std::os::unix::io::RawFd;

pub trait EventHandler {
    fn handle_read(&mut self, fd: RawFd) -> io::Result<()>;
    fn handle_write(&mut self, fd: RawFd) -> io::Result<()>;
}


// TODO: use of deprecated function `nix::sys::epoll::epoll_*`: Use Epoll::new() 

pub struct EventLoop {
    epoll_fd: RawFd,
    handlers: HashMap<RawFd, Box<dyn EventHandler>>,
}

impl EventLoop {
    pub fn new() -> Result<Self> {
        let epoll_fd = epoll_create1(EpollCreateFlags::empty())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(EventLoop {
            epoll_fd,
            handlers: HashMap::new(),
        })
    }

    pub fn add_handler(
        &mut self,
        fd: RawFd,
        events: EpollFlags,
        handler: Box<dyn EventHandler>,
    ) -> Result<()> {
        let mut event = EpollEvent::new(events, fd as u64);

        epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, fd, &mut event)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        self.handlers.insert(fd, handler);
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let mut events = vec![EpollEvent::empty(); 1024];

        loop {
            let num_events = epoll_wait(self.epoll_fd, &mut events, -1)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            for n in 0..num_events {
                let event = events[n];
                let fd = event.data() as RawFd;

                if let Some(handler) = self.handlers.get_mut(&fd) {
                    if event.events().contains(EpollFlags::EPOLLIN) {
                        handler.handle_read(fd)?;
                    }
                    if event.events().contains(EpollFlags::EPOLLOUT) {
                        handler.handle_write(fd)?;
                    }
                }
            }
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.epoll_fd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{io::AsRawFd, net::UnixStream};

    struct TestHandler;

    impl EventHandler for TestHandler {
        fn handle_read(&mut self, _fd: RawFd) -> io::Result<()> {
            Ok(())
        }

        fn handle_write(&mut self, _fd: RawFd) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_event_loop_creation() {
        let event_loop = EventLoop::new();
        assert!(event_loop.is_ok());
    }

    #[test]
    fn test_add_handler() {
        let mut event_loop = EventLoop::new().unwrap();
        let (sock1, _sock2) = UnixStream::pair().unwrap();
        let handler = Box::new(TestHandler);
        let result = event_loop.add_handler(
            sock1.as_raw_fd(),
            EpollFlags::EPOLLIN | EpollFlags::EPOLLOUT,
            handler,
        );
        assert!(result.is_ok());
    }
}
