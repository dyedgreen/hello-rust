use std::io;
use std::io::Write;
mod server;
use server::Server;

fn main() -> io::Result<()> {
    let server = Server::new("127.0.0.1:8000".to_string());
    server.serve(|req, mut res| {
        println!("hit: {:?}", req.location());
        res.header("Content-Type".to_string(), "text/html".to_string())?;
        let body = format!("<h1>Hello There!</h1><h3>{}</h3><p>It works :)</p>", req.location());
        res.write(body.as_bytes())?;

        Ok(())
    })?;

    Ok(())
}
