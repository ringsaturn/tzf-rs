use prost::Message;
use std::fs;

mod pb;
pub use pb::*;

impl TryFrom<String> for Timezones {
    type Error = anyhow::Error;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        let value = fs::read(path).unwrap();
        Ok(Timezones::decode(&value[..])?)
    }
}
