pub enum IrcControl {
    Join { channel: String },
    Leave { channel: String },
    Message { channel: String, message: String },
}

pub enum IrcResponse {
    UserJoined { channel: String, user: String },
}
