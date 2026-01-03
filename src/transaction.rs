use std::fmt;
use std::fmt::Formatter;
use std::io::{BufRead, Write};
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use sha2::Digest;

#[derive(Debug)]
pub enum Error{
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for Error{}

#[derive(Debug)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
    pub lock_time: u32,
}

impl Transaction {
    pub fn txid(&self) -> TxId {
        let mut txid_data = Vec::new();
        self.version.consensus_encode(&mut txid_data).expect("Error serializing txid");
        self.inputs.consensus_encode(&mut txid_data).expect("Error serializing inputs");
        self.outputs.consensus_encode(&mut txid_data).expect("Error serializing outputs");
        self.lock_time.consensus_encode(&mut txid_data).expect("Error serializing locktime");
        TxId::from_raw_transaction(txid_data)
    }
}

impl Serialize for Transaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut tx = serializer.serialize_struct("Transaction", 5)?;
        tx.serialize_field("transaction_id", &self.txid())?;
        tx.serialize_field("version", &self.version)?;
        tx.serialize_field("inputs", &self.inputs)?;
        tx.serialize_field("outputs", &self.outputs)?;
        tx.serialize_field("lock_time", &self.lock_time)?;
        tx.end()
    }
}

#[derive(Debug, Serialize)]
pub struct TxIn {
    pub txid: TxId,
    pub vout: u32,
    pub script_sig: Script,
    pub sequence: u32,
}

#[derive(Debug, Serialize)]
pub struct TxOut {
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
    pub fn from_hash(bytes: [u8; 32]) -> TxId {
        TxId(bytes)
    }

    pub fn from_raw_transaction(tx: Vec<u8>) -> TxId {
        let mut hasher = sha2::Sha256::new();
        hasher.update(tx);
        let hash1 = hasher.finalize();

        let mut hasher = sha2::Sha256::new();
        hasher.update(hash1);
        let hash2 = hasher.finalize();

        TxId::from_hash(hash2.into())
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
    pub fn from_hex(hex: String) -> Script {
        Script(hex::decode(hex).unwrap())
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
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

#[derive(Debug, Serialize)]
pub struct CompactSize(pub u64);

pub trait Decodable : Sized {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error>;
}

impl Decodable for u8 {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let mut buffer = [0u8; 1];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(u8::from_le_bytes(buffer))
    }
}

impl Decodable for u16 {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let mut buffer = [0u8; 2];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(u16::from_le_bytes(buffer))
    }
}

impl Decodable for u32 {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let mut buffer = [0u8; 4];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(u32::from_le_bytes(buffer))
    }
}

impl Decodable for u64 {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let mut buffer = [0u8; 8];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(u64::from_le_bytes(buffer))
    }
}

impl Decodable for CompactSize {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let n = u8::consensus_decode(r)?;

        match n {
            0..=252 => Ok(CompactSize(n as u64)),
            253 => {
                let x = u16::consensus_decode(r)?;
                Ok(CompactSize(x as u64))
            },
            254 => {
                let x = u32::consensus_decode(r)?;
                Ok(CompactSize(x as u64))
            },
            255 => {
                let x = u64::consensus_decode(r)?;
                Ok(CompactSize(x))
            }
        }
    }
}

impl Decodable for String {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let length = CompactSize::consensus_decode(r)?.0;
        let mut buffer = vec![0u8; length as usize];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(hex::encode(buffer))
    }
}

impl Decodable for TxIn {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        Ok(TxIn{
            txid: TxId::consensus_decode(r)?,
            vout: u32::consensus_decode(r)?,
            script_sig: Script::from_hex(String::consensus_decode(r)?),
            sequence: u32::consensus_decode(r)?,
        })
    }
}

impl Decodable for TxOut {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        Ok(TxOut{
            amount: Amount::from_sat(u64::consensus_decode(r)?),
            script_pubkey: Script::from_hex(String::consensus_decode(r)?),
        })
    }
}

impl Decodable for TxId {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let mut buffer = [0u8; 32];
        r.read_exact(&mut buffer).map_err(Error::Io)?;
        Ok(TxId::from_hash(buffer))
    }
}

impl Decodable for Vec<TxIn> {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let input_count = CompactSize::consensus_decode(r)?.0;
        let mut inputs = Vec::with_capacity(input_count as usize);
        for _ in 0..input_count {
            inputs.push(TxIn::consensus_decode(r)?);
        }
        Ok(inputs)
    }
}

impl Decodable for Vec<TxOut> {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let output_count = CompactSize::consensus_decode(r)?.0;
        let mut outputs = Vec::with_capacity(output_count as usize);
        for _ in 0..output_count {
            outputs.push(TxOut::consensus_decode(r)?);
        }
        Ok(outputs)
    }
}

impl Decodable for Transaction {
    fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, Error> {
        Ok(Transaction {
            version: u32::consensus_decode(r)?,
            inputs: Vec::<TxIn>::consensus_decode(r)?,
            outputs: Vec::<TxOut>::consensus_decode(r)?,
            lock_time: u32::consensus_decode(r)?,
        })
    }
}

pub trait Encodable {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error>;
}

impl Encodable for u8 {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let len = w.write([*self].as_slice()).map_err(Error::Io)?;
        Ok(len)
    }
}

impl Encodable for u16 {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let b = self.to_le_bytes();
        let len = w.write(b.as_slice()).map_err(Error::Io)?;
        Ok(len)
    }
}

impl Encodable for u32 {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let b = self.to_le_bytes();
        let len = w.write(b.as_slice()).map_err(Error::Io)?;
        Ok(len)
    }
}

impl Encodable for u64 {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let b = self.to_le_bytes();
        let len = w.write(b.as_slice()).map_err(Error::Io)?;
        Ok(len)
    }
}

impl Encodable for [u8; 32] {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let len = w.write(self.as_slice()).map_err(Error::Io)?;
        Ok(len)
    }
}

impl Encodable for String {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let b = hex::decode(self).expect("should be a valid hex string");
        let compact_size_len = CompactSize(b.len() as u64).consensus_encode(w)?;
        let b_len = w.write(&b).map_err(Error::Io)?;
        Ok(compact_size_len + b_len)
    }
}

impl Encodable for CompactSize {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        match self.0 {
            0..=0xFC => {
                (self.0 as u8).consensus_encode(w)?;
                Ok(1)
            }
            0xFD..=0xFFFF => {
                w.write([0xFD].as_slice()).map_err(Error::Io)?;
                (self.0 as u16).consensus_encode(w)?;
                Ok(3)
            }
            0x10000..=0xFFFFFFFF => {
                w.write([0xFE].as_slice()).map_err(Error::Io)?;
                (self.0 as u32).consensus_encode(w)?;
                Ok(5)
            }
            _ => {
                w.write([0xFF].as_slice()).map_err(Error::Io)?;
                self.0.consensus_encode(w)?;
                Ok(9)
            }
        }
    }
}

impl Encodable for Vec<TxIn> {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let mut len = 0;
        len += CompactSize(self.len() as u64).consensus_encode(w)?;
        for tx in self.iter() {
            len += tx.consensus_encode(w)?;
        }
        Ok(len)
    }
}

impl Encodable for TxId {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        Ok(self.0.consensus_encode(w)?)
    }
}

impl Encodable for Script {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        self.to_hex().consensus_encode(w)
    }
}

impl Encodable for TxIn {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let mut len = 0;
        len += self.txid.consensus_encode(w)?;
        len += self.vout.consensus_encode(w)?;
        len += self.script_sig.consensus_encode(w)?;
        len += self.sequence.consensus_encode(w)?;
        Ok(len)
    }
}

impl Encodable for Vec<TxOut> {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let mut len = 0;
        len += CompactSize(self.len() as u64).consensus_encode(w)?;
        for tx in self.iter() {
            len += tx.consensus_encode(w)?;
        }
        Ok(len)
    }
}

impl Encodable for Amount {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let len = self.0.consensus_encode(w)?;
        Ok(len)
    }
}

impl Encodable for TxOut {
    fn consensus_encode<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        let mut len = 0;
        len += self.amount.consensus_encode(w)?;
        len += self.script_pubkey.consensus_encode(w)?;
        Ok(len)
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use crate::transaction::CompactSize;
    use crate::transaction::Decodable;

    #[test]
    fn read_compact_size_test() -> Result<(), Box<dyn Error>> {
        let mut bytes = [1_u8].as_slice();
        let count = CompactSize::consensus_decode(&mut bytes)?.0;
        assert_eq!(count, 1);

        let mut bytes = [253_u8, 0, 1].as_slice();
        let count = CompactSize::consensus_decode(&mut bytes)?.0;
        assert_eq!(count, 256);

        let mut bytes = [254_u8, 0, 0, 0, 1].as_slice();
        let count = CompactSize::consensus_decode(&mut bytes)?.0;
        assert_eq!(count, 256_u64.pow(3));

        let mut bytes = [255_u8, 0, 0, 0, 0, 0, 0, 0, 1].as_slice();
        let count = CompactSize::consensus_decode(&mut bytes)?.0;
        assert_eq!(count, 256_u64.pow(7));

        let hex = "fd204e";
        let decoded = hex::decode(hex)?;
        let mut bytes = decoded.as_slice();
        let count = CompactSize::consensus_decode(&mut bytes)?.0;
        assert_eq!(count, 20_000);

        Ok(())
    }
}