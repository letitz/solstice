#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlResponse {
    LoginStatusResponse(LoginStatusResponse),
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum LoginStatusResponse {
    LoginOk {
        username: String,
        motd: String,
    },
    LoginFail {
        username: String,
        reason: String,
    }
}
