#[derive(Debug)]
pub enum IrcControl {
    Join { channel: String },
    Part { channel: String },
    Message { channel: String, message: String },
}

#[derive(Debug)]
pub enum IrcEvent {
    ChangeName {
        previous_name: String,
        name: String,
    },
    AddUser {
        channel: String,
        user: String,
        primary: bool,
        joined: bool,
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
