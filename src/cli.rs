use clap::{Parser, Subcommand};
use crate::commands::Commands;

#[derive(Parser)]
#[command(
    name = "recipe",
    version,
    about = "P2P recipe sharing CLI",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
