Things to do:
-------------

 - Switch from `encoding` to `encoding_rs` crate
 - Define error type for ProtoDecoder errors.
 - Remove BytesMut dependency from ProtoEncoder, use Vec<u8> instead.
 - Remove dependency on bytes crate entirely.
 - Handle RoomLeaveRequest/Response.
 - Print out surplus bytes in hex to make analyzing them easier
 - Handle client connections
