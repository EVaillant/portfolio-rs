#[derive(Debug)]
pub enum ErrorKind {
    Referential,
    Persistance,
    Historical,
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(kind: ErrorKind, message: T) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
