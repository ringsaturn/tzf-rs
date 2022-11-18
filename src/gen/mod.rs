use prost::Message;

mod pb;
pub use pb::*;

// impl TryFrom<String> for Timezones {
//     type Error = anyhow::Error;

//     fn try_from(path: String) -> Result<Self, Self::Error> {
//         let value = fs::read(path).unwrap();
//         Ok(Timezones::decode(&value[..])?)
//     }

// }

// impl TryFrom<String> for Timezones {
//     type Error = anyhow::Error;

//     fn try_from(path: String) -> Result<Self, Self::Error> {
//         let value = fs::read(path).unwrap();
//         Ok(Timezones::decode(&value[..])?)
//     }

// }

impl TryFrom<Vec<u8>> for Timezones {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Timezones::decode(&value[..])?)
    }
}
