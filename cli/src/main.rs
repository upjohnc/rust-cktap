/// CLI for rust-cktap
use clap::{Parser, Subcommand};
use rpassword::read_password;
use rust_cktap::commands::{CkTransport, Read};
#[cfg(feature = "emulator")]
use rust_cktap::emulator;
#[cfg(not(feature = "emulator"))]
use rust_cktap::pcsc;
use rust_cktap::secp256k1::rand;
use rust_cktap::{apdu::Error, commands::Certificate, rand_chaincode, CkTapCard};
use std::io;
use std::io::Write;

/// SatsCard CLI
#[derive(Parser)]
#[command(author, version = option_env ! ("CARGO_PKG_VERSION").unwrap_or("unknown"), about,
long_about = None, propagate_version = true)]
struct SatsCardCli {
    #[command(subcommand)]
    command: SatsCardCommand,
}

/// Commands supported by SatsCard cards
#[derive(Subcommand)]
enum SatsCardCommand {
    /// Show the card status
    Debug,
    /// Show current deposit address
    Address,
    /// Check this card was made by Coinkite: Verifies a certificate chain up to root factory key.
    Certs,
    /// Read the pubkey
    Read,
    /// Pick a new private key and start a fresh slot. Current slot must be unsealed.
    New,
    /// Unseal the current slot.
    Unseal,
    /// Get the payment address and verify it follows from the chain code and master public key
    Derive,
    Sign,
}

/// TapSigner CLI
#[derive(Parser)]
#[command(author, version = option_env ! ("CARGO_PKG_VERSION").unwrap_or("unknown"), about,
long_about = None, propagate_version = true)]
struct TapSignerCli {
    #[command(subcommand)]
    command: TapSignerCommand,
}

/// Commands supported by TapSigner cards
#[derive(Subcommand)]
enum TapSignerCommand {
    /// Show the card status
    Debug,
    /// Check this card was made by Coinkite: Verifies a certificate chain up to root factory key.
    Certs,
    /// Read the pubkey (requires CVC)
    Read,
    /// This command is used once to initialize a new card.
    Init,
    /// Derive a public key at the given hardened path
    Derive {
        /// path, eg. for 84'/0'/0'/* use 84,0,0
        #[clap(short, long, value_delimiter = ',', num_args = 1..)]
        path: Vec<u32>,
    },
    Sign {
        #[clap(short, long, value_delimiter = ',', num_args = 1..)]
        path: Vec<u32>,
    },
}

fn main() -> Result<(), Error> {
    // figure out what type of card we have before parsing cli args
    #[cfg(not(feature = "emulator"))]
    let mut card = pcsc::find_first()?;

    // if emulator feature enabled override pcsc card
    #[cfg(feature = "emulator")]
    let mut card = emulator::find_emulator()?;

    let rng = &mut rand::thread_rng();

    match &mut card {
        CkTapCard::SatsCard(sc) => {
            let cli = SatsCardCli::parse();
            match cli.command {
                SatsCardCommand::Debug => {
                    dbg!(&sc);
                }
                SatsCardCommand::Address => println!("Address: {}", sc.address().unwrap()),
                SatsCardCommand::Certs => check_cert(sc),
                SatsCardCommand::Read => read(sc, None),
                SatsCardCommand::New => {
                    let slot = sc.slot().expect("current slot number");
                    let chain_code = Some(rand_chaincode(rng).to_vec());
                    let response = &sc.new_slot(slot, chain_code, cvc()).unwrap();
                    println!("{}", response)
                }
                SatsCardCommand::Unseal => {
                    let slot = sc.slot().expect("current slot number");
                    let response = &sc.unseal(slot, cvc()).unwrap();
                    println!("{}", response)
                }
                SatsCardCommand::Derive => {
                    dbg!(&sc.derive());
                }
                SatsCardCommand::Sign => {
                    dbg!(&sc.sign(cvc()));
                }
            }
        }
        CkTapCard::TapSigner(ts) | CkTapCard::SatsChip(ts) => {
            let cli = TapSignerCli::parse();
            match cli.command {
                TapSignerCommand::Debug => {
                    dbg!(&ts);
                }
                TapSignerCommand::Certs => check_cert(ts),
                TapSignerCommand::Read => read(ts, Some(cvc())),
                TapSignerCommand::Init => {
                    let chain_code = rand_chaincode(rng).to_vec();
                    let response = &ts.init(chain_code, cvc());
                    dbg!(response);
                }
                TapSignerCommand::Derive { path } => {
                    dbg!(&ts.derive(path, cvc()));
                }
                TapSignerCommand::Sign { path } => {
                    dbg!(&ts.sign(path, cvc()));
                }
            }
        }
    }

    Ok(())
}

// handler functions for each command

fn check_cert<T: CkTransport>(card: &mut dyn Certificate<T>) {
    if let Ok(k) = card.check_certificate() {
        println!(
            "Genuine card from Coinkite.\nHas cert signed by: {}",
            k.name()
        )
    } else {
        println!("Card failed to verify. Not a genuine card")
    }
}

fn read<T: CkTransport>(card: &mut dyn Read<T>, cvc: Option<String>) {
    match card.read(cvc) {
        Ok(resp) => println!("{}", resp),
        Err(e) => {
            dbg!(&e);
            println!("Failed to read with error: ")
        }
    }
}

fn cvc() -> String {
    print!("Enter cvc: ");
    io::stdout().flush().unwrap();
    let cvc = read_password().unwrap();
    cvc.trim().to_string()
}
