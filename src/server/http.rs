use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};

const HTTP_VERSION: &'static str = "HTTP/1.1";

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Unknown(String),
}

#[derive(Debug)]
pub struct Request {
    method: Method,
    location: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct Response<W: Write> {
    status: u32,
    headers: HashMap<String, String>,
    dirty: bool,
    socket: W,
}

// Return method, location
fn parse_start_line(line: String) -> io::Result<(Method, String)> {
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() < 3 {
        return Err(Error::new(ErrorKind::InvalidData, "protocol not HTTP"));
    } else if *parts.last().unwrap() != HTTP_VERSION {
        return Err(Error::new(ErrorKind::InvalidData, "invalid HTTP version"));
    }

    return Ok((
        match parts[0] {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            method => Method::Unknown(method.to_string()),
        },
        parts[1].to_string(),
    ));
}

// Parse a line and return the header
fn parse_header_line(line: String) -> io::Result<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(Error::new(ErrorKind::InvalidData, "malformed HTTP header"));
    }
    return Ok((parts[0].trim().to_string(), parts[1].trim().to_string()));
}

fn status_reason(status: u32) -> &'static str {
    match status {
        // information
        100 => &"Continue",
        101 => &"Switching Protocols",
        // success
        200 => &"OK",
        201 => &"Created",
        202 => &"Accepted",
        203 => &"Non-Authoritative Information",
        204 => &"No Content",
        205 => &"Reset Content",
        206 => &"Partial Content",
        // redirection
        300 => &"Multiple Choices",
        301 => &"Moved Permanently",
        302 => &"Found",
        303 => &"See Other",
        304 => &"Not Modified",
        305 => &"Use Proxy",
        307 => &"Temporary Redirect",
        // client error
        400 => &"Bad Request",
        401 => &"Unauthorized",
        402 => &"Payment Required",
        403 => &"Forbidden",
        404 => &"Not Found",
        405 => &"Method Not Allowed",
        406 => &"Not Acceptable",
        407 => &"Proxy Authentication Required",
        408 => &"Request Time-out",
        409 => &"Conflict",
        410 => &"Gone",
        411 => &"Length Required",
        412 => &"Precondition Failed",
        413 => &"Request Entity Too Large",
        414 => &"Request-URI Too Large",
        415 => &"Unsupported Media Type",
        416 => &"Requested range not satisfiable",
        417 => &"Expectation Failed",
        418 => &"I'm a teapot",
        // server error
        500 => &"Internal Server Error",
        501 => &"Not Implemented",
        502 => &"Bad Gateway",
        503 => &"Service Unavailable",
        504 => &"Gateway Time-out",
        505 => &"HTTP Version not supported",
        // other
        _ => &"Unknown Status",
    }
}

impl Method {
    #[allow(dead_code)]
    pub fn to_str(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Unknown(method) => &method,
        }
    }

    #[allow(dead_code)]
    pub fn is_get(&self) -> bool {
        match self {
            Method::Get => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_post(&self) -> bool {
        match self {
            Method::Post => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_put(&self) -> bool {
        match self {
            Method::Put => true,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_delete(&self) -> bool {
        match self {
            Method::Delete => true,
            _ => false,
        }
    }
}

impl Request {
    /// Read request from incoming tcp stream and
    /// return the assembled request.
    pub fn from_stream<T: Read>(stream: T) -> io::Result<Request> {
        let mut req = Request {
            method: Method::Get,
            location: String::new(),
            headers: HashMap::new(),
            body: None,
        };

        let mut buf = BufReader::new(stream);

        // Parse headers
        let mut started = false;
        for line in (&mut buf).lines() {
            let line = line?;

            // parse starting line
            if !started {
                if line.is_empty() {
                    // consume empty lines at start
                    // (this is recommend in spec)
                    continue;
                }
                let (method, location) = parse_start_line(line)?;
                req.method = method;
                req.location = location;
                started = true;
                continue;
            }

            // parse lines until one is empty
            if line.is_empty() {
                break;
            }
            let (key, val) = parse_header_line(line)?;
            req.headers.insert(key, val);
        }

        // Read body if Content-Length > 0
        if req.headers.contains_key("Content-Length") {
            let content_len: usize = req
                .headers
                .get("Content-Length")
                .unwrap()
                .parse()
                .or_else(|_| Err(Error::new(ErrorKind::InvalidData, "invalid content length")))?;
            if content_len > 0 {
                let mut body = vec![];
                body.resize(content_len, 0);
                let read_len = buf.read(&mut body)?;
                body.resize(read_len, 0);
                req.body = Some(body);
            }
        }

        Ok(req)
    }

    #[allow(dead_code)]
    pub fn method(&self) -> &Method {
        &self.method
    }

    #[allow(dead_code)]
    pub fn location(&self) -> &String {
        &self.location
    }

    #[allow(dead_code)]
    pub fn header(&self, key: &String) -> Option<&String> {
        self.headers.get(key)
    }

    #[allow(dead_code)]
    pub fn body(&self) -> Option<String> {
        self.body
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }

    #[allow(dead_code)]
    pub fn body_bytes(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }
}

impl<W: Write> Response<W> {
    /// Construct a new response which writes to socket
    pub fn for_stream(socket: W) -> Response<W> {
        let mut resp = Response {
            status: 200,
            headers: HashMap::new(),
            dirty: false,
            socket,
        };
        resp.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        resp
    }

    /// Set the specified status
    #[allow(dead_code)]
    pub fn status(&mut self, status: u32) -> io::Result<()> {
        if status >= 600 {
            return Err(Error::new(ErrorKind::InvalidData, "invalid status code"));
        } else if self.dirty {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "status already written to client",
            ));
        }
        self.status = status;
        Ok(())
    }

    /// Set the specified header
    #[allow(dead_code)]
    pub fn header(&mut self, key: String, val: String) -> io::Result<()> {
        if self.dirty {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "status already written to client",
            ));
        }
        self.headers.insert(key, val);
        Ok(())
    }
}

impl<W: Write> Write for Response<W> {
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut written = 0;
        if !self.dirty {
            // Send headers
            let mut head = format!(
                "{} {} {}\r\n",
                HTTP_VERSION,
                self.status,
                status_reason(self.status)
            );
            for (key, val) in self.headers.iter() {
                head.push_str(&format!("{}: {}\r\n", key, val));
            }
            head.push_str("\r\n");
            written += self.socket.write(head.as_bytes())?;
            self.dirty = true;
        }
        // Send buffer
        written += self.socket.write(buf)?;
        Ok(written)
    }
}
