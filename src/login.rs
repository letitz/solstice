/// Represents the status of the login operation.
///
/// In order to interact with the server, a client cannot simply open a network
/// connection. Instead, it must send a login request with basic credentials.
/// The server is supposed the respond with a success or error message. Once
/// successfully logged in, the client can interact with the server.
#[derive(Clone, Debug)]
pub enum LoginStatus {
  /// Request not sent yet.
  Todo,

  /// Sent request, awaiting response.
  AwaitingResponse,

  /// Logged in.
  /// Stores the MOTD as received from the server.
  Success(String),

  /// Failed to log in.
  /// Stores the error message as received from the server.
  Failure(String),
}
