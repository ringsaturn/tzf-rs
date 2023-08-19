use prost::Message;

pub mod pb;
pub use pb::*;

impl TryFrom<Vec<u8>> for Timezones {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self::decode(&value[..])?)
    }
}

impl TryFrom<Vec<u8>> for PreindexTimezones {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self::decode(&value[..])?)
    }
}
