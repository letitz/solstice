use std::io;
use std::io::{Read, Write};

use rustc_serialize::json;

use mio::tcp::TcpStream;

#[derive(RustcDecodable, RustcEncodable)]
pub enum ControlRequest {
    LoginRequest(LoginRequest),
}

impl ControlRequest {
    fn read_from<R: Read + Sized>(&self, mut reader: R) -> io::Result<Self> {
        let mut string = String::new();
        try!(reader.read_to_string(&mut string));
        match json::decode(&string) {
            Ok(request) => Ok(request),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub enum ControlResponse {
    LoginResponse(LoginResponse),
}

impl ControlResponse {
    fn write_to<W: Write + Sized>(&self, mut writer: W) -> io::Result<()> {
        match json::encode(self) {
            Ok(json_string) => {
                try!(writer.write(&json_string.into_bytes()));
                Ok(())
            },

            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

pub struct ControlStream {
    stream: TcpStream,
}

impl ControlStream {
    pub fn new(stream: TcpStream) -> Self {
        ControlStream {
            stream: stream,
        }
    }

    pub fn read_request(&mut self) -> io::Result<Option<ControlRequest>> {
        Ok(None)
    }

    pub fn write_response(&mut self, response: &ControlResponse)
        -> io::Result<()>
    {
        response.write_to(&mut self.stream)
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(RustcDecodable, RustcEncodable)]
pub enum LoginResponse {
    LoginOk {
        motd: String,
    },
    LoginFail {
        reason: String,
    }
}
