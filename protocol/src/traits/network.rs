use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

use crate::types::Bytes;
use crate::{async_trait, codec::ProtocolCodec, traits::Context, Display, ProtocolResult};

#[derive(Clone, Debug, Copy)]
pub enum Priority {
    High,
    Normal,
}

#[derive(Debug, Display, Clone)]
pub enum TrustFeedback {
    #[display(fmt = "fatal {}", _0)]
    Fatal(String),
    #[display(fmt = "worse {}", _0)]
    Worse(String),
    #[display(fmt = "bad {}", _0)]
    Bad(String),
    #[display(fmt = "neutral")]
    Neutral,
    #[display(fmt = "good")]
    Good,
}

#[derive(Debug, Display, Clone)]
pub enum PeerTag {
    #[display(fmt = "consensus")]
    Consensus,
    #[display(fmt = "always allow")]
    AlwaysAllow,
    #[display(fmt = "banned, until {}", until)]
    Ban { until: u64 }, // timestamp
    #[display(fmt = "{}", _0)]
    Custom(String), // TODO: Hide custom constructor
}

impl PeerTag {
    pub fn ban(until: u64) -> Self {
        PeerTag::Ban { until }
    }

    pub fn ban_key() -> Self {
        PeerTag::Ban { until: 0 }
    }

    pub fn custom<S: AsRef<str>>(s: S) -> Option<Self> {
        let custom_str = s.as_ref();
        match custom_str {
            "consensus" | "always_allow" | "ban" => None,
            _ => Some(PeerTag::Custom(custom_str.to_owned())),
        }
    }

    pub fn str(&self) -> &str {
        match self {
            PeerTag::Consensus => "consensus",
            PeerTag::AlwaysAllow => "always_allow",
            PeerTag::Ban { .. } => "ban",
            PeerTag::Custom(str) => str,
        }
    }
}

impl PartialEq for PeerTag {
    fn eq(&self, other: &PeerTag) -> bool {
        self.str() == other.str()
    }
}

impl Eq for PeerTag {}

impl Hash for PeerTag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.str().hash(state)
    }
}

pub trait MessageCodec: Sized + Send + Debug + 'static {
    fn encode_msg(&mut self) -> ProtocolResult<Bytes>;

    fn decode_msg(bytes: Bytes) -> ProtocolResult<Self>;
}

impl<T: ProtocolCodec + Debug + 'static> MessageCodec for T {
    fn encode_msg(&mut self) -> ProtocolResult<Bytes> {
        self.encode()
    }

    fn decode_msg(bytes: Bytes) -> ProtocolResult<Self> {
        Self::decode(bytes)
    }
}

#[async_trait]
pub trait Gossip: Send + Sync {
    async fn broadcast<M>(&self, cx: Context, end: &str, msg: M, p: Priority) -> ProtocolResult<()>
    where
        M: MessageCodec;

    async fn multicast<'a, M, P>(
        &self,
        cx: Context,
        end: &str,
        peer_ids: P,
        msg: M,
        p: Priority,
    ) -> ProtocolResult<()>
    where
        M: MessageCodec,
        P: AsRef<[Bytes]> + Send + 'a;
}

#[async_trait]
pub trait Rpc: Send + Sync {
    async fn call<M, R>(&self, ctx: Context, end: &str, msg: M, pri: Priority) -> ProtocolResult<R>
    where
        M: MessageCodec,
        R: MessageCodec;

    async fn response<M>(
        &self,
        cx: Context,
        end: &str,
        ret: ProtocolResult<M>,
        p: Priority,
    ) -> ProtocolResult<()>
    where
        M: MessageCodec;
}

pub trait Network: Send + Sync {
    fn tag(&self, ctx: Context, peer_id: Bytes, tag: PeerTag) -> ProtocolResult<()>;
    fn untag(&self, ctx: Context, peer_id: Bytes, tag: &PeerTag) -> ProtocolResult<()>;
    fn tag_consensus(&self, ctx: Context, peer_ids: Vec<Bytes>) -> ProtocolResult<()>;
    fn peer_count(&self, ctx: Context) -> ProtocolResult<usize>;
}

pub trait PeerTrust: Send + Sync {
    fn report(&self, ctx: Context, feedback: TrustFeedback);
}

#[async_trait]
pub trait MessageHandler: Sync + Send + 'static {
    type Message: MessageCodec;

    async fn process(&self, ctx: Context, msg: Self::Message) -> TrustFeedback;
}
