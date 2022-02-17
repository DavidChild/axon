use ethcore::client::BlockId;
use hash::H256;
use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// Represents rpc api block number param.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum Web3BlockNumber {
    Hash {
        hash:              Hash,
        /// only return blocks part of the canon chain
        require_canonical: bool,
    },
    Num(u64),
    Latest,
    Earliest,
    Pending,
}

impl Default for Web3BlockNumber {
    fn default() -> Self {
        Web3BlockNumber::Latest
    }
}

impl<'a> Deserialize<'a> for Web3BlockNumber {
    fn deserialize<D>(deserializer: D) -> Result<Web3BlockNumber, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(Web3BlockNumberVisitor)
    }
}

impl Web3BlockNumber {
    /// Convert block number to min block target.
    pub fn to_min_block_num(&self) -> Option<u64> {
        match *self {
            Web3BlockNumber::Num(ref x) => Some(*x),
            _ => None,
        }
    }
}

impl Serialize for Web3BlockNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Web3BlockNumber::Hash {
                hash,
                require_canonical,
            } => serializer.serialize_str(&format!(
                "{{ 'hash': '{}', 'requireCanonical': '{}'  }}",
                hash, require_canonical
            )),
            Web3BlockNumber::Num(ref x) => serializer.serialize_str(&format!("0x{:x}", x)),
            Web3BlockNumber::Latest => serializer.serialize_str("latest"),
            Web3BlockNumber::Earliest => serializer.serialize_str("earliest"),
            Web3BlockNumber::Pending => serializer.serialize_str("pending"),
        }
    }
}

struct Web3BlockNumberVisitor;

impl<'a> Visitor<'a> for Web3BlockNumberVisitor {
    type Value = Web3BlockNumber;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a block number or 'latest', 'earliest' or 'pending'"
        )
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'a>,
    {
        let (mut require_canonical, mut block_number, mut block_hash) =
            (false, None::<u64>, None::<H256>);

        loop {
            let key_str: Option<String> = visitor.next_key()?;

            match key_str {
                Some(key) => match key.as_str() {
                    "Web3BlockNumber" => {
                        let value: String = visitor.next_value()?;
                        if value.starts_with("0x") {
                            let number = u64::from_str_radix(&value[2..], 16).map_err(|e| {
                                Error::custom(format!("Invalid block number: {}", e))
                            })?;

                            block_number = Some(number);
                            break;
                        } else {
                            return Err(Error::custom(
                                "Invalid block number: missing 0x prefix".to_string(),
                            ));
                        }
                    }
                    "blockHash" => {
                        block_hash = Some(visitor.next_value()?);
                    }
                    "requireCanonical" => {
                        require_canonical = visitor.next_value()?;
                    }
                    key => return Err(Error::custom(format!("Unknown key: {}", key))),
                },
                None => break,
            };
        }

        if let Some(number) = block_number {
            return Ok(Web3BlockNumber::Num(number));
        }

        if let Some(hash) = block_hash {
            return Ok(Web3BlockNumber::Hash {
                hash,
                require_canonical,
            });
        }

        return Err(Error::custom("Invalid input"));
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match value {
            "latest" => Ok(Web3BlockNumber::Latest),
            "earliest" => Ok(Web3BlockNumber::Earliest),
            "pending" => Ok(Web3BlockNumber::Pending),
            _ if value.starts_with("0x") => u64::from_str_radix(&value[2..], 16)
                .map(Web3BlockNumber::Num)
                .map_err(|e| Error::custom(format!("Invalid block number: {}", e))),
            _ => Err(Error::custom(
                "Invalid block number: missing 0x prefix".to_string(),
            )),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

/// Converts `Web3BlockNumber` to `BlockId`, panics on
/// `Web3BlockNumber::Pending`
pub fn block_number_to_id(number: Web3BlockNumber) -> BlockId {
    match number {
        Web3BlockNumber::Hash { hash, .. } => BlockId::Hash(hash),
        Web3BlockNumber::Num(num) => BlockId::Number(num),
        Web3BlockNumber::Earliest => BlockId::Earliest,
        Web3BlockNumber::Latest => BlockId::Latest,
        Web3BlockNumber::Pending => panic!("`Web3BlockNumber::Pending` should be handled manually"),
    }
}
