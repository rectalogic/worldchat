#[derive(Debug)]
pub enum IrcControlMessage {
    Part,
    Message { message: String },
}

#[derive(Debug)]
pub enum IrcEvent {
    PrimaryUser { nick: String, name: String },
    AddUser { nick: String },
    ChangeName { previous_nick: String, nick: String },
    UserJoined { nick: String },
    Part { nick: String },
    Quit { nick: String },
    Message { nick: String, message: String },
}
