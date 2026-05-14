#[derive(Debug)]
pub enum IrcControlMessage {
    Part,
    Message { message: String },
}
