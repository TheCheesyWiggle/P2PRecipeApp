#[derive(Subcommand)]
pub enum PeerCommand {
    List,
    Connect {
        peer_id: String,
    },
    Disconnect {
        peer_id: String,
    },
}
