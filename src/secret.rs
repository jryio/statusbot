use std::fmt::{Debug, Display};

pub struct Secret(pub String);

impl Secret {
    pub fn new(val: String) -> Self {
        Self(val)
    }
}

impl Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SECRET_CANNOT_BE_LOGGED")
    }
}
impl Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SECRET_CANNOT_BE_LOGGED")
    }
}
