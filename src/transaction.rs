use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub transaction_id: TxId,
    pub version: u32,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub lock_time: u32,
}

#[derive(Debug, Serialize)]
pub struct Input {
    pub txid: TxId,
    pub vout: u32,
    pub script_sig: Script,
    pub sequence: u32,
}

#[derive(Debug, Serialize)]
pub struct Output {
    #[serde(serialize_with = "as_btc")]
    pub amount: Amount,
    pub script_pubkey: Script,
}

#[derive(Debug, Serialize)]
pub struct Amount(u64);

impl Amount {
    pub fn from_sat(satoshi: u64) -> Amount {
        Amount(satoshi)
    }
}

#[derive(Debug)]
pub struct TxId([u8; 32]);

impl TxId {
    pub fn from_bytes(bytes: [u8; 32]) -> TxId {
        TxId(bytes)
    }
}

impl Serialize for TxId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut bytes = self.0.clone();
        bytes.reverse();
        serializer.serialize_str(&hex::encode(&bytes))
    }
}

#[derive(Debug)]
pub struct Script(Vec<u8>);

impl Script {
    pub fn from_vec(vec: Vec<u8>) -> Script {
        Script(vec)
    }
}

impl Serialize for Script {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        serializer.serialize_str(&hex::encode(self.0.as_slice()))
    }
}

impl BitcoinValue for Amount {
    fn to_btc(&self) -> f64{
        self.0 as f64 / 100_000_000.0
    }
}

trait BitcoinValue {
    fn to_btc(&self) -> f64;
}

fn as_btc<S: Serializer, T: BitcoinValue>(t : &T, s: S) -> Result<S::Ok, S::Error> {
    let btc = t.to_btc();
    s.serialize_f64(btc)
}

