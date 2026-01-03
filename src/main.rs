use clap::Parser;

#[derive(Parser)]
#[command(name = "Transaction Decoder")]
#[command(version = "1.0.0")]
#[command(about = "Bitcoin Transaction Decoder")]
struct Cli {
    #[arg(required = true, help = "(string) hex of the transaction to decode")]
    transaction_hex: String,
}

fn main() {
    let cli = Cli::parse();
    match transaction_decoder::decode(cli.transaction_hex) {
        Ok(decoded) => println!("Transaction: {}", decoded),
        Err(error) => eprintln!("{}", error)
    }
}