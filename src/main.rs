mod transaction;

use std::io::{Error, Read};
use std::error::Error as StdError;
use sha2::Digest;
use transaction::{Amount, Input, Output, Transaction};
use crate::transaction::{Script, TxId};

fn read_compact_size(transaction_bytes: &mut &[u8]) -> Result<u64, Error> {
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

fn decode(transaction_hex: String) -> Result<String, Box<dyn StdError>>{
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
            output_index,
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

fn main() {
    let transaction_hex = "01000000015180fff4155787703d10f03cca1566794516ac65a67764e571dc9c34931f321d050000006a473044022100a98648381f405a6882989faa500147c7cb9f4ce03e912d18529fb3609e243a47021f798214efe634e8c47e158edae534f5652b98ce1bd3693fa95dcdd2c699d987012102bd63ab2a6215bdd16d554ea3fd5d83843bff9c76e0b8c6c150d58ee6ca7ea525ffffffff01a08b2013000000001976a9142f58e6245481be77894d5f0f0e2641decdafc44788ac00000000";
    match decode(transaction_hex.to_string()) {
        Ok(decoded) => println!("Transaction: {}", decoded),
        Err(error) => eprintln!("{}", error)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use crate::read_compact_size;

    #[test]
    fn read_compact_size_test() -> Result<(), Box<dyn Error>> {
        let mut bytes = [1_u8].as_slice();
        let count = read_compact_size(&mut bytes)?;
        assert_eq!(count, 1);

        let mut bytes = [253_u8, 0, 1].as_slice();
        let count = read_compact_size(&mut bytes)?;
        assert_eq!(count, 256);

        let mut bytes = [254_u8, 0, 0, 0, 1].as_slice();
        let count = read_compact_size(&mut bytes)?;
        assert_eq!(count, 256_u64.pow(3));

        let mut bytes = [255_u8, 0, 0, 0, 0, 0, 0, 0, 1].as_slice();
        let count = read_compact_size(&mut bytes)?;
        assert_eq!(count, 256_u64.pow(7));

        let hex = "fd204e";
        let decoded = hex::decode(hex)?;
        let mut bytes = decoded.as_slice();
        let count = read_compact_size(&mut bytes)?;
        assert_eq!(count, 20_000);

        Ok(())
    }
}
