#[derive(Subcommand)]
pub enum ConfigCommand {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
    List,
}
