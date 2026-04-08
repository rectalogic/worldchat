#[derive(Debug)]
pub enum IrcControl {
    Join { channel: String },
    Part { channel: String },
    Message { channel: String, message: String },
}

#[derive(Debug)]
pub enum IrcEvent {
    Nick {
        server_user: String,
    },
    Join {
        channel: String,
        user: String,
    },
    Part {
        channel: String,
        user: String,
    },
    Quit {
        user: String,
    },
    Message {
        channel: String,
        user: String,
        message: String,
    },
}
