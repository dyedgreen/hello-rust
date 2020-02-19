// use server::ServePool;
// use http::Status;

use std::io;
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};

mod pool;
use pool::Pool;
mod http;
use http::{Request, Response};

fn handle_stream(stream: TcpStream) -> io::Result<()> {
    println!("stream: {:?}", stream);

    let req = Request::from_stream(&stream)?;
    println!("method: {:?}", req.method());
    println!("location: {:?}", req.location());
    println!("body: {:?}", req.body());

    let mut resp = Response::for_stream(&stream);
    resp.header("Content-Type".to_string(), "text/html".to_string())?;
    resp.write("<h1>Hello World!</h1><p>It works :)</p>".as_bytes())?;
    stream.shutdown(Shutdown::Both)?;

    Ok(())
}

fn main() -> io::Result<()> {
    // // Create a new pool that will spawn 10 threads
    // let pool = ServePool::new(10);

    // // Handle incoming requests
    // pool.handle(|req, res| -> Status {
    //     res.write("Hello world!");
    //     // maybe (?)
    //     Status::Ok(200)
    // });

    // // Start pool
    // pool.serve("127.0.0.1:9898");

    let pool = Pool::new(2);

    let listener = TcpListener::bind("127.0.0.1:8000")?;
    for stream in listener.incoming() {
        let stream = stream?;
        pool.schedule(|| {
            println!("{:?}", handle_stream(stream));
        });
    }
    Ok(())
}
