use std::fmt;

use jsonrpsee::core::DeserializeOwned;
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{from_value, Value};

use core_consensus::SyncStatus as InnerSyncStatus;
use protocol::codec::ProtocolCodec;
use protocol::types::{
    AccessList, Block, Bloom, Bytes, Hash, Hex, Public, Receipt, SignedTransaction, H160, H256,
    U256, U64,
};

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum RichTransactionOrHash {
    Hash(Hash),
    Rich(SignedTransaction),
}

impl Serialize for RichTransactionOrHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            RichTransactionOrHash::Hash(h) => h.serialize(serializer),
            RichTransactionOrHash::Rich(stx) => stx.serialize(serializer),
        }
    }
}

impl RichTransactionOrHash {
    pub fn get_hash(&self) -> Hash {
        match self {
            RichTransactionOrHash::Hash(hash) => *hash,
            RichTransactionOrHash::Rich(stx) => stx.transaction.hash,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Web3Transaction {
    pub block_number:             U256,
    pub block_hash:               H256,
    pub from:                     H160,
    pub contract_address:         Option<H160>,
    pub cumulative_gas_used:      U256,
    pub effective_gas_price:      U256,
    pub gas:                      U256,
    pub creates:                  Option<H160>,
    pub raw:                      Hex,
    pub public_key:               Option<Public>,
    pub gas_price:                U256,
    pub max_fee_per_gas:          U256,
    pub max_priority_fee_per_gas: U256,
    pub hash:                     Hash,
    pub input:                    Hex,
    pub nonece:                   U256,
    pub to:                       Option<H160>,
    pub transaction_index:        Option<U256>,
    pub value:                    U256,
    #[serde(rename = "type")]
    pub type_:                    Option<U64>,
    pub access_list:              Option<AccessList>,
    pub chain_id:                 Option<U256>,
    pub standard_v:               Option<U256>,
    pub r:                        U256,
    pub s:                        U256,
}

impl Web3Transaction {
    pub fn create(receipt: Receipt, stx: SignedTransaction) -> Web3Transaction {
        let signature = stx.transaction.signature.clone();
        let mut web3_transaction_out_tx = Web3Transaction {
            block_number:             receipt.block_number.into(),
            block_hash:               receipt.block_hash,
            from:                     receipt.sender,
            contract_address:         receipt.code_address.map(Into::into),
            cumulative_gas_used:      receipt.used_gas,
            effective_gas_price:      receipt.used_gas,
            creates:                  receipt.code_address.map(Into::into),
            raw:                      Hex::encode(stx.transaction.encode().unwrap()),
            public_key:               stx.public,
            gas:                      receipt.used_gas,
            gas_price:                stx.transaction.unsigned.gas_price,
            max_fee_per_gas:          U256::from(1337u64),
            max_priority_fee_per_gas: stx.transaction.unsigned.max_priority_fee_per_gas,
            hash:                     receipt.tx_hash,
            to:                       stx.get_to(),
            input:                    Hex::encode(stx.transaction.unsigned.data),
            nonece:                   stx.transaction.unsigned.value,
            transaction_index:        Some(receipt.tx_index.into()),
            value:                    stx.transaction.unsigned.value,
            type_:                    Some(0x02u64.into()),
            access_list:              Some(stx.transaction.unsigned.access_list.clone()),
            chain_id:                 Some(stx.transaction.chain_id.into()),
            standard_v:               Some(U256::default()),
            r:                        U256::default(),
            s:                        U256::default(),
        };
        if let Some(sc) = signature {
            web3_transaction_out_tx.standard_v = Some(sc.standard_v.into());
            web3_transaction_out_tx.r = sc.r.as_ref().into();
            web3_transaction_out_tx.s = sc.s.as_ref().into();
        }
        web3_transaction_out_tx
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Web3Receipt {
    pub block_number:        U256,
    pub block_hash:          H256,
    pub contract_address:    Option<H160>,
    pub cumulative_gas_used: U256,
    pub effective_gas_price: U256,
    pub from:                H160,
    pub gas_used:            U256,
    pub logs:                Vec<Web3ReceiptLog>,
    pub logs_bloom:          Bloom,
    #[serde(rename = "root")]
    pub state_root:          Hash,
    pub status:              U256,
    pub to:                  Option<H160>,
    pub transaction_hash:    Hash,
    pub transaction_index:   Option<U256>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type:    Option<U64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Web3ReceiptLog {
    pub address:           H160,
    pub topics:            Vec<H256>,
    pub data:              Hex,
    pub block_number:      U256,
    pub transaction_hash:  Hash,
    pub transaction_index: Option<U256>,
    pub block_hash:        Hash,
    pub log_index:         U256,
    pub removed:           bool,
}

impl Web3Receipt {
    pub fn new(receipt: Receipt, stx: SignedTransaction) -> Web3Receipt {
        let mut web3_receipt = Web3Receipt {
            block_number:        receipt.block_number.into(),
            block_hash:          receipt.block_hash,
            contract_address:    receipt.code_address.map(Into::into),
            cumulative_gas_used: receipt.used_gas,
            effective_gas_price: receipt.used_gas,
            from:                receipt.sender,
            status:              receipt.status(),
            gas_used:            receipt.used_gas,
            logs:                vec![],
            logs_bloom:          receipt.logs_bloom,
            state_root:          receipt.state_root,
            to:                  stx.get_to(),
            transaction_hash:    receipt.tx_hash,
            transaction_index:   Some(receipt.tx_index.into()),
            transaction_type:    Some(0x02u64.into()),
        };
        for item in receipt.logs.into_iter() {
            web3_receipt.logs.push(Web3ReceiptLog {
                address:           item.address,
                topics:            item.topics,
                data:              Hex::encode(item.data),
                block_number:      receipt.block_number.into(),
                transaction_hash:  receipt.tx_hash,
                transaction_index: Some(receipt.tx_index.into()),
                block_hash:        receipt.block_hash,
                // Todo: FIX ME
                log_index:         U256::default(),
                // Todo: FIXME
                removed:           false,
            });
        }
        web3_receipt
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Web3Block {
    pub hash:              H256,
    pub parent_hash:       H256,
    #[serde(rename = "sha3Uncles")]
    pub sha3_uncles:       H256,
    pub author:            H160,
    pub miner:             H160,
    pub state_root:        H256,
    pub transactions_root: H256,
    pub receipts_root:     H256,
    pub number:            U256,
    pub gas_used:          U256,
    pub gas_limit:         U256,
    pub extra_data:        Hex,
    pub logs_bloom:        Option<Bloom>,
    pub timestamp:         U256,
    pub difficulty:        U256,
    pub total_difficulty:  Option<U256>,
    pub seal_fields:       Vec<Bytes>,
    pub base_fee_per_gas:  U256,
    pub uncles:            Vec<H256>,
    pub transactions:      Vec<RichTransactionOrHash>,
    pub size:              Option<U256>,
    pub mix_hash:          H256,
    pub nonce:             U256,
}

impl From<Block> for Web3Block {
    fn from(b: Block) -> Self {
        Web3Block {
            hash:              b.header_hash(),
            number:            b.header.number.into(),
            author:            b.header.proposer,
            parent_hash:       b.header.prev_hash,
            sha3_uncles:       Default::default(),
            logs_bloom:        Some(b.header.log_bloom),
            transactions_root: b.header.transactions_root,
            state_root:        b.header.state_root,
            receipts_root:     b.header.receipts_root,
            miner:             b.header.proposer,
            difficulty:        b.header.difficulty,
            total_difficulty:  None,
            seal_fields:       vec![],
            base_fee_per_gas:  b.header.base_fee_per_gas,
            extra_data:        Hex::encode(&b.header.extra_data),
            size:              Some(b.header.size().into()),
            gas_limit:         b.header.gas_limit,
            gas_used:          b.header.gas_used,
            timestamp:         b.header.timestamp.into(),
            transactions:      b
                .tx_hashes
                .iter()
                .map(|hash| RichTransactionOrHash::Hash(*hash))
                .collect(),
            uncles:            vec![],
            mix_hash:          H256::default(),
            nonce:             U256::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransactionCondition {
    #[serde(rename = "block")]
    Number(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Web3CallRequest {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type:         Option<U64>,
    pub from:                     Option<H160>,
    pub to:                       H160,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price:                Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas:          Option<U256>,
    pub gas:                      Option<U256>,
    pub value:                    Option<U256>,
    pub data:                     Hex,
    pub nonce:                    Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list:              Option<AccessList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WEB3Work {
    /// The proof-of-work hash.
    pub pow_hash:  H256,
    /// The seed hash.
    pub seed_hash: H256,
    /// The target.
    pub target:    H256,
    /// The block number: this isn't always stored.
    pub number:    Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockId {
    Num(u64),
    Hash(H256),
    Latest,
}

impl Default for BlockId {
    fn default() -> Self {
        BlockId::Latest
    }
}

impl From<BlockId> for Option<u64> {
    fn from(id: BlockId) -> Self {
        match id {
            BlockId::Num(num) => Some(num),
            BlockId::Latest => None,
            BlockId::Hash(_h) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Web3BlockNumber {
    Hash {
        hash:              H256,
        require_canonical: bool,
    },

    Num(u64),

    Latest,
    // Earliest,
    Pending,
}

impl<'a> Deserialize<'a> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<BlockId, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(BlockIdVisitor)
    }
}

impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            BlockId::Num(ref x) => serializer.serialize_str(&format!("0x{:x}", x)),
            BlockId::Latest => serializer.serialize_str("latest"),
            BlockId::Hash(hash) => serializer.serialize_str(&format!(
                "{{ 'hash': '{}', 'requireCanonical': '{}'  }}",
                hash, false
            )),
        }
    }
}

struct BlockIdVisitor;

impl<'a> Visitor<'a> for BlockIdVisitor {
    type Value = BlockId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a block number or 'latest' ")
    }

    #[allow(clippy::never_loop)]
    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'a>,
    {
        let mut block_number = None;

        loop {
            let key_str: Option<String> = visitor.next_key()?;

            match key_str {
                Some(key) => match key.as_str() {
                    "blockNumber" => {
                        let value: String = visitor.next_value()?;
                        if let Some(stripper) = value.strip_prefix("0x") {
                            let number = u64::from_str_radix(stripper, 16).map_err(|e| {
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
                    key => return Err(Error::custom(format!("Unknown key: {}", key))),
                },
                None => break,
            };
        }

        if let Some(number) = block_number {
            return Ok(BlockId::Num(number));
        }

        Err(Error::custom("Invalid input"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match value {
            "latest" => Ok(BlockId::Latest),
            _ if value.starts_with("0x") => u64::from_str_radix(&value[2..], 16)
                .map(BlockId::Num)
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

#[derive(Debug, PartialEq)]
pub struct Index(usize);

impl Index {
    pub fn value(&self) -> usize {
        self.0
    }
}

impl<'a> Deserialize<'a> for Index {
    fn deserialize<D>(deserializer: D) -> Result<Index, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_any(IndexVisitor)
    }
}

struct IndexVisitor;

impl<'a> Visitor<'a> for IndexVisitor {
    type Value = Index;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a hex-encoded or decimal index")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match value {
            _ if value.starts_with("0x") => usize::from_str_radix(&value[2..], 16)
                .map(Index)
                .map_err(|e| Error::custom(format!("Invalid index: {}", e))),
            _ => value
                .parse::<usize>()
                .map(Index)
                .map_err(|e| Error::custom(format!("Invalid index: {}", e))),
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Web3Filter {
    pub from_block: Option<BlockId>,
    pub to_block:   Option<BlockId>,
    pub block_hash: Option<H256>,
    pub address:    Option<H160>,
    pub topics:     Option<Vec<H256>>,
    pub limit:      Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Web3Log {
    pub address:           H160,
    pub topics:            Vec<H256>,
    pub data:              Hex,
    pub block_hash:        Option<H256>,
    pub block_number:      Option<U256>,
    pub transaction_hash:  Option<H256>,
    pub transaction_index: Option<U256>,
    pub log_index:         Option<U256>,
    #[serde(default)]
    pub removed:           bool,
    #[serde(rename = "type")]
    pub log_type:          String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Web3SyncStatus {
    Doing(SyncStatus),
    False,
}

impl From<InnerSyncStatus> for Web3SyncStatus {
    fn from(inner: InnerSyncStatus) -> Self {
        match inner {
            InnerSyncStatus::False => Web3SyncStatus::False,
            InnerSyncStatus::Syncing {
                start,
                current,
                highest,
            } => Web3SyncStatus::Doing(SyncStatus {
                starting_block: start,
                current_block:  current,
                highest_block:  highest,
                known_states:   U256::default(),
                pulled_states:  U256::default(),
            }),
        }
    }
}

impl Serialize for Web3SyncStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Web3SyncStatus::Doing(status) => status.serialize(serializer),
            Web3SyncStatus::False => false.serialize(serializer),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub starting_block: U256,
    pub current_block:  U256,
    pub highest_block:  U256,
    pub known_states:   U256,
    pub pulled_states:  U256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Web3FeeHistory {
    pub oldest_block:     U256,
    pub reward:           Option<Vec<U256>>,
    pub base_fee_per_gas: Vec<U256>,
    pub gas_used_ratio:   Vec<U256>,
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
            // Web3BlockNumber::Earliest => serializer.serialize_str("earliest"),
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
            //  "earliest" => Ok(Web3BlockNumber::Earliest),
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

#[derive(Serialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum VariadicValue<T>
where
    T: DeserializeOwned,
{
    /// Single
    Single(T),
    /// List
    Multiple(Vec<T>),
    /// None
    Null,
}
impl<'a, T> Deserialize<'a> for VariadicValue<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<VariadicValue<T>, D::Error>
    where
        D: Deserializer<'a>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;

        if v.is_null() {
            return Ok(VariadicValue::Null);
        }

        from_value(v.clone())
            .map(VariadicValue::Single)
            .or_else(|_| from_value(v).map(VariadicValue::Multiple))
            .map_err(|err| D::Error::custom(format!("Invalid variadic value type: {}", err)))
    }
}
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub from_block: BlockId,
    pub to_block:   BlockId,
    pub address:    Option<Vec<H160>>,
    pub topics:     Vec<Option<Vec<H256>>>,
    pub limit:      Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChangeWeb3Filter {
    pub from_block: Option<Web3BlockNumber>,
    pub to_block:   Option<Web3BlockNumber>,
    pub block_hash: Option<H256>,
    pub address:    Option<VariadicValue<H160>>,
    pub topics:     Option<Vec<VariadicValue<H256>>>,
    pub limit:      Option<usize>,
}

impl ChangeWeb3Filter {
    pub fn try_into(self) -> Filter {
        let num_to_id = |num| match num {
            Web3BlockNumber::Hash { hash, .. } => BlockId::Hash(hash),
            Web3BlockNumber::Num(n) => BlockId::Num(n),
            // Web3BlockNumber::Earliest => BlockId::Earliest,
            Web3BlockNumber::Latest | Web3BlockNumber::Pending => BlockId::Latest,
        };

        let (from_block, to_block) = match self.block_hash {
            Some(hash) => (BlockId::Hash(hash), BlockId::Hash(hash)),
            None => (
                self.from_block.map_or_else(|| BlockId::Latest, &num_to_id),
                self.to_block.map_or_else(|| BlockId::Latest, &num_to_id),
            ),
        };

        Filter {
            from_block,
            to_block,
            address: self.address.and_then(|address| match address {
                VariadicValue::Null => None,
                VariadicValue::Single(a) => Some(vec![a]),
                VariadicValue::Multiple(a) => Some(a),
            }),
            topics: {
                let mut iter = self
                    .topics
                    .map_or_else(Vec::new, |topics| {
                        topics
                            .into_iter()
                            .take(4)
                            .map(|topic| match topic {
                                VariadicValue::Null => None,
                                VariadicValue::Single(t) => Some(vec![t]),
                                VariadicValue::Multiple(t) => Some(t),
                            })
                            .collect()
                    })
                    .into_iter();

                vec![
                    iter.next().unwrap_or(None),
                    iter.next().unwrap_or(None),
                    iter.next().unwrap_or(None),
                    iter.next().unwrap_or(None),
                ]
            },
            limit: self.limit,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FilterChanges {
    Logs(Vec<Web3Log>),
    Hashes(Vec<H256>),
    Empty,
}

impl Serialize for FilterChanges {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            FilterChanges::Logs(ref logs) => logs.serialize(s),
            FilterChanges::Hashes(ref hashes) => hashes.serialize(s),
            FilterChanges::Empty => (&[] as &[Value]).serialize(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_json() {
        let status = Web3SyncStatus::False;
        let json = json::parse(&serde_json::to_string(&status).unwrap()).unwrap();
        assert!(json.is_boolean());

        let status = Web3SyncStatus::Doing(SyncStatus {
            starting_block: fastrand::u64(..).into(),
            current_block:  fastrand::u64(..).into(),
            highest_block:  fastrand::u64(..).into(),
            known_states:   U256::default(),
            pulled_states:  U256::default(),
        });
        let json = json::parse(&serde_json::to_string(&status).unwrap()).unwrap();
        assert!(json.is_object());
    }
}
