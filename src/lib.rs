use crate::transaction::Decodable;
mod transaction;

use std::error::Error as StdError;
use transaction::Transaction;

pub fn decode(transaction_hex: String) -> Result<String, Box<dyn StdError>>{
    let transaction_bytes = hex::decode(transaction_hex)?;
    let transaction = Transaction::consensus_decode(&mut transaction_bytes.as_slice())?;
    Ok(serde_json::to_string_pretty(&transaction)?)
}