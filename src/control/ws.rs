use std::error;
use std::fmt;

use crossbeam_channel;
use rustc_serialize::json;
use ws;

use crate::config;

use super::request::*;
use super::response::*;

/// This enum contains the possible notifications that the control loop will
/// send to the client.
#[derive(Debug)]
pub enum Notification {
  /// A new controller has connected: control messages can now be sent on the
  /// given channel.
  Connected(Sender),
  /// The controller has disconnected.
  Disconnected,
  /// An irretrievable error has arisen.
  Error(String),
  /// The controller has sent a request.
  Request(Request),
}

/// This error is returned when a `Sender` fails to send a control request.
#[derive(Debug)]
pub enum SendError {
  /// Error encoding the control request.
  JSONEncoderError(json::EncoderError),
  /// Error sending the encoded control request to the websocket.
  WebSocketError(ws::Error),
}

impl fmt::Display for SendError {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      SendError::JSONEncoderError(ref err) => {
        write!(fmt, "JSONEncoderError: {}", err)
      }
      SendError::WebSocketError(ref err) => {
        write!(fmt, "WebSocketError: {}", err)
      }
    }
  }
}

impl error::Error for SendError {
  fn description(&self) -> &str {
    match *self {
      SendError::JSONEncoderError(_) => "JSONEncoderError",
      SendError::WebSocketError(_) => "WebSocketError",
    }
  }

  fn cause(&self) -> Option<&dyn error::Error> {
    match *self {
      SendError::JSONEncoderError(ref err) => Some(err),
      SendError::WebSocketError(ref err) => Some(err),
    }
  }
}

impl From<json::EncoderError> for SendError {
  fn from(err: json::EncoderError) -> Self {
    SendError::JSONEncoderError(err)
  }
}

impl From<ws::Error> for SendError {
  fn from(err: ws::Error) -> Self {
    SendError::WebSocketError(err)
  }
}

/// This struct is used to send control responses to the controller.
/// It encapsulates the websocket connection so as to isolate clients from
/// the underlying implementation.
#[derive(Clone, Debug)]
pub struct Sender {
  sender: ws::Sender,
}

impl Sender {
  /// Queues up a control response to be sent to the controller.
  pub fn send(&mut self, response: Response) -> Result<(), SendError> {
    let encoded = json::encode(&response)?;
    self.sender.send(encoded)?;
    Ok(())
  }
}

/// This struct handles a single websocket connection.
#[derive(Debug)]
struct Handler {
  /// The channel on which to send notifications to the client.
  client_tx: crossbeam_channel::Sender<Notification>,
  /// The channel on which to send messages to the controller.
  socket_tx: ws::Sender,
}

impl Handler {
  fn send_to_client(&self, notification: Notification) -> ws::Result<()> {
    match self.client_tx.send(notification) {
      Ok(()) => Ok(()),
      Err(e) => {
        error!("Error sending notification to client: {}", e);
        Err(ws::Error::new(ws::ErrorKind::Internal, ""))
      }
    }
  }
}

impl ws::Handler for Handler {
  fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
    info!("Websocket open");
    self.send_to_client(Notification::Connected(Sender {
      sender: self.socket_tx.clone(),
    }))
  }

  fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
    info!("Websocket closed: code: {:?}, reason: {:?}", code, reason);
    self
      .send_to_client(Notification::Disconnected)
      .unwrap_or(())
  }

  fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
    // Get the payload string.
    let payload = match msg {
      ws::Message::Text(payload) => payload,
      ws::Message::Binary(_) => {
        error!("Received binary websocket message from controller");
        return Err(ws::Error::new(
          ws::ErrorKind::Protocol,
          "Binary message not supported",
        ));
      }
    };

    // Decode the json control request.
    let control_request = match json::decode(&payload) {
      Ok(control_request) => control_request,
      Err(e) => {
        error!("Received invalid JSON message from controller: {}", e);
        return Err(ws::Error::new(ws::ErrorKind::Protocol, "Invalid JSON"));
      }
    };

    debug!("Received control request: {:?}", control_request);

    // Send the control request to the client.
    self.send_to_client(Notification::Request(control_request))
  }
}

/// Start listening on the socket address stored in configuration, and send
/// control notifications to the client through the given channel.
pub fn listen(client_tx: crossbeam_channel::Sender<Notification>) {
  let websocket_result = ws::Builder::new()
    .with_settings(ws::Settings {
      max_connections: 1,
      ..ws::Settings::default()
    })
    .build(|socket_tx| Handler {
      client_tx: client_tx.clone(),
      socket_tx: socket_tx,
    });

  let websocket = match websocket_result {
    Ok(websocket) => websocket,
    Err(e) => {
      error!("Unable to build websocket: {}", e);
      client_tx
        .send(Notification::Error(format!(
          "Unable to build websocket: {}",
          e
        )))
        .unwrap();
      return;
    }
  };

  let listen_result =
    websocket.listen((config::CONTROL_HOST, config::CONTROL_PORT));

  match listen_result {
    Ok(_) => (),
    Err(e) => {
      error!("Unable to listen on websocket: {}", e);
      client_tx
        .send(Notification::Error(format!(
          "Unable to listen on websocket: {}",
          e
        )))
        .unwrap();
    }
  }
}
