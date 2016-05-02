pub const MAX_PACKET_SIZE: usize = 1 << 20; // 1 MiB
pub const U32_SIZE: usize = 4;
pub const MAX_MESSAGE_SIZE: usize = MAX_PACKET_SIZE - U32_SIZE;

pub const MAX_PORT: u32 = (1 << 16) - 1;

