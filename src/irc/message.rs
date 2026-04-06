pub enum IrcControl {
    Join { channel: String },
    Part { channel: String },
    Message { channel: String, message: String },
}

pub enum IrcEvent {
    UserJoined { channel: String, user: String },
    UserParted { channel: String, user: String },
    UserQuit { user: String },
}
