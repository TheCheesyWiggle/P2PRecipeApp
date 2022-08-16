use std::lazy::Lazy;

const STORAGE_FILE_PATH:&str = "./recipes.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
//allows for subscriptions to specific computers??
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

type Recipes = Vec<Recipe>;

#[derive(Debug, Serialize, Deserialize)]
struct Recipe {
    id: usize,
    name: String,
    ingredients: String,
    instructions: String,
    public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse {
    mode: ListMode,
    data: Recipes,
    receiver: String,
}

enum EventType {
    Response(ListResponse),
    Input(String),
}
#[tokio]
async fn main() {
    //initalizes logger
    pretty_env_logger::init();

    info!("Peer ID: {}",PEER_ID.clone());
    //creates channel for communication within the application
    let (response_sender, mut response_crv) = mpsc::unbounded_channel();
    //keypair for the noise protocol
    let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&KEYS).expect("Can create auth keys");

    //Creates transport which is a feature of the libp2p framework
    let transport = TokioTcpConfig::new()
        //Upgrades version of the transport once connection is established as Version 1 of the multistream-select protocol is the version that interacts with the noise protocol
        //in short handles protocol negotiation
        .upgrade(upgrade::Version::V1)
        //Authenticates that channel is secure with the noise XX handshake
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        //multiplex transport negotiates multiple sub-streams and/or connections on the authenticated transport
        .multiplex(mplex::MplexConfig::new())
        //boxed allows only output and error types to be captured
        .boxed();

    //dictates network behaviour
    let behaviour = RecipeBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        //mdns protocol automatically discovers peers and adds them too the network
        mdns: TokioMdns::new().expect("Can create mdns"),
        repsonse_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());

    //manages connections created using transport and executes using the network behaviour
    let mut swarm = SwarmBuilder::new(transport, behaviour, PEER_ID.clone())
        //executor tell swarm to use the tokio runtime
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();
    //starts the swarm
    Swarm::listen_on(
        //lets os pick decide a port
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0".parse().expect("Can get local socket"),
    ).expect("Can start swarm");

}
