use prost::Message;

pub mod tzf {
    pub mod v1 {
        include!("tzf.v1.rs");
    }
}
pub use tzf::v1::*;

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
