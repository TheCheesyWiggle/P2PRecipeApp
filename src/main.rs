use libp2p::{
    Swarm,
    PeerId,
    Multiaddr,
    swarm::{NetworkBehaviour, SwarmEvent},
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    mdns,
    tcp,
    yamux,
    noise,
    identity,
};


use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};

fn storage_file_path() -> String {
    std::env::var("STORAGE_FILE_PATH").unwrap_or_else(|_| "./recipes.json".to_string())
}

type RecipeResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
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

#[derive(NetworkBehaviour)]
struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: mdns::tokio::Behaviour,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer ID: {}", PEER_ID.clone());

    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    // Create swarm with modern libp2p APIs
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(KEYS.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("Failed to create transport")
        .with_behaviour(|key| {
            let peer_id = PeerId::from(key.public());
            RecipeBehaviour {
                floodsub: Floodsub::new(peer_id),
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    peer_id,
                )
                .expect("Failed to create mDNS behaviour"),
            }
        })
        .expect("Failed to create behaviour")
        .build();

    swarm.behaviour_mut().floodsub.subscribe(TOPIC.clone());

    let port: u16 = std::env::var("P2P_PORT")
        .unwrap_or_else(|_| "4001".to_string())
        .parse()
        .expect("P2P_PORT must be a valid u16");
    
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port)
        .parse()
        .expect("Can parse listen addr");
    
    swarm.listen_on(listen_addr).expect("Can start swarm");

    if let Ok(peers_str) = std::env::var("BOOTSTRAP_PEERS") {
        for peer_addr in peers_str.split(',') {
            let peer_addr = peer_addr.trim();
            if !peer_addr.is_empty() {
                match peer_addr.parse::<Multiaddr>() {
                    Ok(addr) => {
                        let _ = swarm.dial(addr);
                    }
                    Err(e) => {
                        error!("Invalid bootstrap peer address: {} - {}", peer_addr, e);
                    }
                }
            }
        }
    }

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    let storage_path = storage_file_path();
    if !std::path::Path::new(&storage_path).exists() {
        write_local_recipes(&vec![])
            .await
            .expect("Failed to create recipes.json");
    }

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(line)) = line {
                        Some(EventType::Input(line))
                    } else {
                        None
                    }
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(RecipeBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, _multiaddr) in list {
                                info!("mDNS discovered a new peer: {}", peer_id);
                                swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                            }
                            None
                        }
                        SwarmEvent::Behaviour(RecipeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, _multiaddr) in list {
                                info!("mDNS peer expired: {}", peer_id);
                                swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                            }
                            None
                        }
                        SwarmEvent::Behaviour(RecipeBehaviourEvent::Floodsub(
                            FloodsubEvent::Message(msg),
                        )) => {
                            if let Ok(resp) = serde_json::from_slice::<ListResponse>(&msg.data) {
                                if resp.receiver == PEER_ID.to_string() {
                                    info!("Response from: {}", msg.source);
                                    resp.data.iter().for_each(|r| info!("{:?}", r));
                                }
                            } else if let Ok(req) = serde_json::from_slice::<ListRequest>(&msg.data) {
                                match req.mode {
                                    ListMode::ALL => {
                                        info!("Received ALL req: {:?} from {:?}", req, msg.source);
                                        respond_with_public_recipes(
                                            response_sender.clone(),
                                            msg.source.to_string(),
                                        );
                                    }
                                    ListMode::One(ref peer_id) => {
                                        if peer_id == &PEER_ID.to_string() {
                                            info!("Received req: {:?} from {:?}", req, msg.source);
                                            respond_with_public_recipes(
                                                response_sender.clone(),
                                                msg.source.to_string(),
                                            );
                                        }
                                    }
                                }
                            }
                            None
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {:?}", address);
                            None
                        }
                        _ => None,
                    }
                }
                response = response_rcv.recv() => Some(EventType::Response(response.expect("Response exists"))),
            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    let json = serde_json::to_vec(&resp).expect("can jsonify request");
                    swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), json);
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    cmd if cmd.starts_with("ls r") => handle_list_recipes(cmd, &mut swarm).await,
                    cmd if cmd.starts_with("create r") => handle_create_recipes(cmd).await,
                    cmd if cmd.starts_with("publish r") => handle_publish_recipes(cmd).await,
                    _ => {
                        info!("Unknown command: {}", line);
                        info!("Available commands: ls p | ls r | ls r all | create r | publish r");
                    }
                },
            }
        }
    }
}

async fn handle_list_peers(swarm: &mut Swarm<RecipeBehaviour>) {
    info!("Discovered peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|p| info!("{}", p));
}

async fn handle_create_recipes(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("create r") {
        let rest = rest.trim();
        let elements: Vec<&str> = rest.split('|').collect();
        if elements.len() < 3 {
            info!("too few arguments - Format: name|ingredients|instructions")
        } else {
            let name = elements[0];
            let ingredients = elements[1];
            let instructions = elements[2];
            if let Err(e) = create_new_recipe(name, ingredients, instructions).await {
                error!("error creating recipe: {}", e);
            }
        }
    }
}

async fn create_new_recipe(name: &str, ingredients: &str, instructions: &str) -> RecipeResult<()> {
    let mut local_recipes = read_local_recipes().await?;
    let new_id = match local_recipes.iter().max_by_key(|r| r.id) {
        Some(v) => v.id + 1,
        None => 0,
    };
    local_recipes.push(Recipe {
        id: new_id,
        name: name.to_owned(),
        ingredients: ingredients.to_owned(),
        instructions: instructions.to_owned(),
        public: false,
    });

    write_local_recipes(&local_recipes).await?;
    info!("Created Recipe:");
    info!("Name: {:?}", name);
    info!("Ingredients: {:?}", ingredients);
    info!("Instructions: {:?}", instructions);
    Ok(())
}

async fn handle_publish_recipes(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("publish r") {
        match rest.trim().parse::<usize>() {
            Ok(id) => {
                if let Err(e) = publish_recipe(id).await {
                    info!("error publishing recipe with id {}, {}", id, e);
                } else {
                    info!("Successful publication with id {}", id);
                }
            }
            Err(e) => error!("Invalid id {}, {}", rest.trim(), e),
        }
    }
}

async fn publish_recipe(id: usize) -> RecipeResult<()> {
    let mut local_recipes = read_local_recipes().await?;
    local_recipes
        .iter_mut()
        .filter(|r| r.id == id)
        .for_each(|r| r.public = true);
    write_local_recipes(&local_recipes).await?;
    Ok(())
}

async fn read_local_recipes() -> RecipeResult<Recipes> {
    let path = storage_file_path();
    match fs::read(&path).await {
        Ok(content) => {
            let result = serde_json::from_slice(&content)?;
            Ok(result)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(Box::new(e)),
    }
}

async fn write_local_recipes(recipes: &Recipes) -> RecipeResult<()> {
    let path = storage_file_path();
    let json = serde_json::to_string(&recipes)?;
    fs::write(&path, &json).await?;
    Ok(())
}

async fn handle_list_recipes(cmd: &str, swarm: &mut Swarm<RecipeBehaviour>) {
    let rest = cmd.strip_prefix("ls r ");
    match rest {
        Some("all") => {
            let req = ListRequest {
                mode: ListMode::ALL,
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.into_bytes());
        }
        Some(recipes_peer_id) => {
            let req = ListRequest {
                mode: ListMode::One(recipes_peer_id.to_owned()),
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.into_bytes());
        }
        None => match read_local_recipes().await {
            Ok(v) => {
                info!("Local recipes ({})", v.len());
                v.iter().for_each(|r| info!("{:?}", r))
            }
            Err(e) => error!("error fetching local recipes: {}", e),
        },
    }
}

fn respond_with_public_recipes(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
    tokio::spawn(async move {
        match read_local_recipes().await {
            Ok(recipes) => {
                let resp = ListResponse {
                    mode: ListMode::ALL,
                    receiver,
                    data: recipes.into_iter().filter(|r| r.public).collect(),
                };
                if let Err(e) = sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
        }
    });
}