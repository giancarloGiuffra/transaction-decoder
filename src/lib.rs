mod transaction;

use std::io::{Error, Read};
use std::error::Error as StdError;
use sha2::Digest;
use transaction::{
    Amount,
    Input,
    Output,
    Transaction,
    Script,
    TxId
};

pub fn read_compact_size(transaction_bytes: &mut &[u8]) -> Result<u64, Error> {
    let mut compact_size = [0;1];
    transaction_bytes.read(&mut compact_size)?;

    match compact_size[0] {
        0..=252 => Ok(compact_size[0] as u64),
        253 => {
            let mut buffer = [0;2];
            transaction_bytes.read(&mut buffer)?;
            Ok(u16::from_le_bytes(buffer) as u64)
        },
        254 => {
            let mut buffer = [0;4];
            transaction_bytes.read(&mut buffer)?;
            Ok(u32::from_le_bytes(buffer) as u64)
        },
        255 => {
            let mut buffer = [0;8];
            transaction_bytes.read(&mut buffer)?;
            Ok(u64::from_le_bytes(buffer))
        }
    }
}

fn read_u32(transaction_bytes: &mut &[u8]) -> Result<u32, Error> {
    let mut version_bytes = [0;4];
    transaction_bytes.read(&mut version_bytes)?;
    Ok(u32::from_le_bytes(version_bytes))
}

fn read_amount(transaction_bytes: &mut &[u8]) -> Result<Amount, Error> {
    let mut version_bytes = [0;8];
    transaction_bytes.read(&mut version_bytes)?;
    Ok(Amount::from_sat(u64::from_le_bytes(version_bytes)))
}

fn read_txid(transaction_bytes: &mut &[u8]) -> Result<TxId, Error> {
    let mut txid_bytes = [0;32];
    transaction_bytes.read(&mut txid_bytes)?;
    Ok(TxId::from_bytes(txid_bytes))
}

fn read_script(transaction_bytes: &mut &[u8]) -> Result<Script, Error> {
    let script_size = read_compact_size(transaction_bytes)?;
    let mut script_sig = vec![0; script_size as usize];
    transaction_bytes.read(&mut script_sig)?;
    Ok(Script::from_vec(script_sig))
}

fn hash_raw_transaction(raw_transaction: &[u8]) -> TxId {
    let mut hasher = sha2::Sha256::new();
    hasher.update(raw_transaction);
    let hash1 = hasher.finalize();

    let mut hasher = sha2::Sha256::new();
    hasher.update(hash1);
    let hash2 = hasher.finalize();

    TxId::from_bytes(hash2.into())
}

pub fn decode(transaction_hex: String) -> Result<String, Box<dyn StdError>>{
    let transaction_bytes = hex::decode(transaction_hex)?;
    let mut bytes_slice = transaction_bytes.as_slice();

    let version = read_u32(&mut bytes_slice)?;

    let input_count = read_compact_size(&mut bytes_slice)?;
    let mut inputs = vec![];
    for _ in 0..input_count {
        let txid = read_txid(&mut bytes_slice)?;
        let output_index = read_u32(&mut bytes_slice)?;
        let script_sig = read_script(&mut bytes_slice)?;
        let sequence = read_u32(&mut bytes_slice)?;

        inputs.push(Input {
            txid,
            vout: output_index,
            script_sig,
            sequence,
        });
    }

    let output_count = read_compact_size(&mut bytes_slice)?;
    let mut outputs = vec![];
    for _ in 0..output_count {
        let amount = read_amount(&mut bytes_slice)?;
        let script_pubkey = read_script(&mut bytes_slice)?;

        outputs.push(Output {
            amount,
            script_pubkey
        });
    }

    let lock_time = read_u32(&mut bytes_slice)?;
    let transaction_id = hash_raw_transaction(&transaction_bytes);

    let transaction = Transaction {
        transaction_id,
        version,
        inputs,
        outputs,
        lock_time
    };
    Ok(serde_json::to_string_pretty(&transaction)?)
}