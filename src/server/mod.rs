use std::io;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
mod pool;
use pool::Pool;
mod http;
use http::{Request, Response};

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Server {
        Server { addr }
    }

    /// Block current thread and server incoming
    /// http requests.
    pub fn serve<F>(&self, handle: F) -> io::Result<()>
    where
        F: Fn(Request, Response<TcpStream>) -> io::Result<()> + Sync + Send + Copy + 'static,
    {
        let listener = TcpListener::bind(&self.addr)?;
        let pool = Pool::new(8); // todo: make configurable?

        for conn in listener.incoming() {
            let conn = conn?;
            pool.schedule(move || {
                let req = Request::from_stream(&conn);
                let mut res = Response::for_stream(conn);

                if let Err(error) = req {
                    eprintln!("Invalid request: {}", error);
                    res.status(400).unwrap();
                    if let Err(error) = res.write(&vec![]) {
                        eprintln!(
                            "Further error encountered when sending error status: {}",
                            error
                        );
                    }
                    return;
                }

                match handle(req.unwrap(), res) {
                    Ok(_) => (),
                    Err(error) => eprintln!("Error handling request: {}", error),
                }
            });
        }

        Ok(())
    }
}
