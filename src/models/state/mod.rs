pub mod state_naive;
pub mod state_v01;
pub use state_naive::StateNaive;
pub use state_v01::StateV01;

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::de::{Deserialize, Deserializer, Error as DeserializeError};
use serde::ser::{Error as SerializeError, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use super::{Convert, LinkMetadata, PredicateLayout};
use crate::Error;
use crate::Result;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, EnumIter)]
pub enum StateVersion {
    Naive,
    V0_1,
}

impl Convert<String> for StateVersion {
    fn try_from(target: String) -> Result<Self> {
        match target.as_str() {
            "link" => Ok(StateVersion::Naive),
            "https://in-toto.io/Statement/v0.1" => Ok(StateVersion::V0_1),
            _ => Err(Error::StringConvertFailed(target)),
        }
    }

    fn try_into(self) -> Result<String> {
        match self {
            StateVersion::Naive => Ok("link".to_string()),
            StateVersion::V0_1 => Ok("https://in-toto.io/Statement/v0.1".to_string()),
        }
    }
}

impl Serialize for StateVersion {
    fn serialize<S>(&self, ser: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let target = &self
            .try_into()
            .map_err(|e| SerializeError::custom(format!("{:?}", e)))?;
        ser.serialize_str(target)
    }
}

impl<'de> Deserialize<'de> for StateVersion {
    fn deserialize<D: Deserializer<'de>>(de: D) -> ::std::result::Result<Self, D::Error> {
        let target: String = Deserialize::deserialize(de)?;
        StateVersion::try_from(target).map_err(|e| DeserializeError::custom(format!("{:?}", e)))
    }
}

impl Display for StateVersion {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match self {
            StateVersion::V0_1 => fmt.write_str("v0.1")?,
            StateVersion::Naive => fmt.write_str("naive")?,
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum StateWrapper {
    Naive(StateNaive),
    V0_1(StateV01),
}

pub trait FromMerge: Sized {
    fn merge(meta: LinkMetadata, predicate: Option<Box<dyn PredicateLayout>>) -> Result<Self>;
}

impl StateWrapper {
    pub fn into_trait(self) -> Box<dyn StateLayout> {
        match self {
            StateWrapper::Naive(link) => Box::new(link),
            StateWrapper::V0_1(link) => Box::new(link),
        }
    }

    pub fn from_meta(
        meta: LinkMetadata,
        predicate: Option<Box<dyn PredicateLayout>>,
        version: StateVersion,
    ) -> Self {
        match version {
            StateVersion::Naive => Self::Naive(StateNaive::merge(meta, predicate).unwrap()),
            StateVersion::V0_1 => Self::V0_1(StateV01::merge(meta, predicate).unwrap()),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>, version: StateVersion) -> Result<Self> {
        match version {
            StateVersion::Naive => serde_json::from_slice(&bytes)
                .map(Self::Naive)
                .map_err(|e| e.into()),
            StateVersion::V0_1 => serde_json::from_slice(&bytes)
                .map(Self::V0_1)
                .map_err(|e| e.into()),
        }
    }

    pub fn judge_from_bytes(bytes: Vec<u8>) -> Result<StateVersion> {
        StateVersion::iter()
            .find(|ver| StateWrapper::from_bytes(bytes.clone(), *ver).is_ok())
            .ok_or_else(|| Error::Programming("no available bytes parser".to_string()))
    }

    pub fn try_from_bytes(bytes: Vec<u8>) -> Result<Self> {
        println!("{:?}", bytes);
        Self::judge_from_bytes(bytes.clone())
            .map(|ver| StateWrapper::from_bytes(bytes.clone(), ver).unwrap())
    }
}

pub trait StateLayout {
    fn version(&self) -> StateVersion;
    fn into_enum(self: Box<Self>) -> StateWrapper;
    fn to_bytes(&self) -> Result<Vec<u8>>;
}
