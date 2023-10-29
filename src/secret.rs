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

impl PartialEq<String> for Secret {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Secret> for String {
    fn eq(&self, other: &Secret) -> bool {
        *self == *other.0
    }
}
