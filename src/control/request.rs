#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlRequest {
    LoginStatusRequest(LoginStatusRequest),
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct LoginStatusRequest;
