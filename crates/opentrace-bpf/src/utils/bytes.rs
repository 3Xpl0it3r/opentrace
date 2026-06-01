use std::fmt::Display;

pub(crate) struct Bytes(pub(crate) u32);

impl Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1024 * 1024 {
            write!(f, "{}M", self.0 / (1024 * 1024))
        } else if self.0 >= 1024 {
            write!(f, "{}k", self.0 / 1024)
        } else {
            write!(f, "{}B", self.0)
        }
    }
}
