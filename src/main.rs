extern crate core;

//dependencies
use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise::{Keypair, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport, Multiaddr,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, };
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};

//file path for recipes
const STORAGE_FILE_PATH:&str = "./recipes.json";
//creates Result type with box which doesnt use any heap memory if T is zero but allocates any variables onto the heap
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
//generates keys
static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
//creates peer id
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
//allows for subscriptions to specific peers??
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));
//creates recipes type out of a list of the recipe type
type Recipes = Vec<Recipe>;

//defining structs
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
struct RecipeBehaviour{
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<ListResponse>,
}

//network behaviour defines what bytes and where to send them from the local node for MDNS event
impl NetworkBehaviourEventProcess<MdnsEvent> for RecipeBehaviour{
    //defines how the message is send to the network behaviour from the network handler
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            //triggered when a new peer is discovered on the network
            MdnsEvent::Discovered(discovered_list)=>{
                //for every peer in the multi address in discovered list
                for(peer, _addr) in discovered_list{
                    //adds node to the list of nodes to propagate messages to.
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            //triggered when the records time to live expires and the address hasn’t been refreshed and is removed from the list
            MdnsEvent::Expired(expired_list)=>{
                //for every peer in the multi address in expired list
                for(peer, _addr) in expired_list{
                    //true if the given PeerId is in the list of nodes discovered through mDNS
                    if !self.mdns.has_node(&peer){
                        //removes node from the list of nodes to propagate messages to.
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

//network behaviour defines what bytes and where to send them from the local node for FloodsubEvent
impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour{
    //defines how the message is send to the network behaviour from the network handler
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            //case for a response
            if let Ok(resp) = serde_json::from_slice::<ListResponse>(&msg.data){
                //checks if its indeed for local machine
                if resp.receiver == PEER_ID.to_string(){
                    //output
                    info!("Response from: {}",msg.source);
                    //iterates and outputs the data
                    resp.data.iter().for_each(|r| info!("{:?}",r))
                }
            }
            //case for request
            else if let Ok(req) = serde_json::from_slice::<ListResponse>(&msg.data) {
                //match statement to determine the mode
                match req.mode {
                    //mode all
                    ListMode::ALL => {
                        //outputs requests made
                        info!("Received ALL req: {:?} from {:?}",req,msg.source);
                        //responds with local messages
                        respond_with_public_recipes(
                            self.response_sender.clone(),
                            msg.source.to_string(),
                        );
                    }
                    //mode one
                    ListMode::One(ref peer_id) => {
                        //checks if request is for local machine
                        if peer_id == &PEER_ID.to_string(){
                            //outputs requests
                            info!("Received req: {:?} from {:?}",req,msg.source);
                            respond_with_public_recipes(
                                self.response_sender.clone(),
                                msg.source.to_string(),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    //initializes logger
    pretty_env_logger::init();

    info!("Peer ID: {}",PEER_ID.clone());
    //creates channel for communication within the application
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    //keypair for the noise protocol
    let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&KEYS).expect("Can create auth keys");

    //Creates transport which is a feature of the lib p2p framework
    let transport = TokioTcpConfig::new()
        //Upgrades version of the transport once connection is established as Version 1 of the multi-stream-select protocol is the version that interacts with the noise protocol
        //in short handles protocol negotiation
        .upgrade(upgrade::Version::V1)
        //Authenticates that channel is secure with the noise XX handshake
        .authenticate(libp2p::noise::NoiseConfig::xx(auth_keys).into_authenticated())
        //multiplex transport negotiates multiple sub-streams and/or connections on the authenticated transport
        .multiplex(mplex::MplexConfig::new())
        //boxed allows only output and error types to be captured
        .boxed();

    //dictates network behaviour
    let mut behaviour = RecipeBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        //mdns protocol automatically discovers peers and adds them too the network
        mdns: Mdns::new(Default::default()).await.expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());

    //manages connections created using transport and executes using the network behaviour
    let mut swarm = SwarmBuilder::new(transport, behaviour, PEER_ID.clone())
        //executor tell swarm to use the tokio runtime
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    // choose listen port from env (default 4001)
    let port = std::env::var("P2P_PORT").unwrap_or_else(|_| "4001".to_string());
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port)
        .parse()
        .expect("Can parse listen addr");
    Swarm::listen_on(&mut swarm, listen_addr.clone()).expect("Can start swarm");

    // Optional bootstrap: BOOTSTRAP_PEERS="peer1:4001,peer2:4002"
    if let Ok(peers) = std::env::var("BOOTSTRAP_PEERS") {
        for p in peers.split(',').filter(|s| !s.is_empty()) {
            let mut parts = p.split(':');
            if let (Some(host), Some(port)) = (parts.next(), parts.next()) {
                let ma = format!("/dns4/{}/tcp/{}", host, port)
                    .parse::<Multiaddr>()
                    .expect("bad multiaddr");
                match swarm.dial_addr(ma) {
                    Ok(_) => info!("Dialed bootstrap peer {}", p),
                    Err(e) => error!("failed to dial {}: {}", p, e),
                }
            }
        }
    }


    //allows the async reader to read the lines one by one
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    //event loop processes events from the swarm by listening through stdin
    loop{
        let evt = {
            //select macro waits for several async processes and handles the first one that finishes
            tokio::select!{
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                event = swarm.next() =>{
                    info!("Unhandled Swarm event: {:?}",event);
                    None
                },
                response = response_rcv.recv() => Some(EventType::Response(response.expect("Response exists"))),
            }
        };
        //commands (user interaction)
        if let Some(event) = evt {
            //match statement checks if it is an input or response event
            match event {
                EventType::Response(_resp) => {}
                //if its a input event match again to verify the command
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    cmd if cmd.starts_with("ls r") => handle_list_recipes(cmd, &mut swarm).await,
                    cmd if cmd.starts_with("create r") => handle_create_recipes(cmd).await,
                    cmd if cmd.starts_with("publish r") => handle_publish_recipes(cmd).await,
                    _ => panic!("Unknown command"),
                }
            }
        }
    }
}
//logic for listing peers
async fn handle_list_peers(swarm: &mut Swarm<RecipeBehaviour>){
    info!("Discovered peers:");
    //mdns shows all discovered nodes
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    //adds peers from list to hash set data structure which prevents duplicate values
    for peer in nodes {
        unique_peers.insert(peer);
    }
    //iterates through the hashset and displays the peers
    unique_peers.iter().for_each(|p| info!("{}",p));

}
//logic for handling recipe creation
async fn handle_create_recipes(cmd :&str){
    //removes the command from the string
    if let Some(rest) = cmd.strip_prefix("create r"){
        //splits arguments and stores their references in a array
        let elements: Vec<&str> = rest.split("|").collect();
        //Uses the len function to check number of args
        if elements.len() < 3{
            info!("too few arguments - Format: name|ingredients|instructions")
        }
        else {
            //assigns varible names to arguments
            let name = elements.get(0).expect("name is there");
            let ingredients = elements.get(1).expect("ingredients are there");
            let instructions = elements.get(2).expect("instructions are there");
            //Uses Err enum to handle errors while creating recipes
            if let  Err(e)= create_new_recipe(name,ingredients,instructions).await{
                panic!("error creating recipe: {}", e);
            };
        }
    }
}
//logic for creating a recipe
async fn create_new_recipe(name :&str,ingredients:&str,instructions :&str) -> Result<()>{
    //Creates a list of recipe structs
    let mut local_recipes = read_local_recipes().await?;
    //loops trough local recipes and assigns appropriate id
    //if no local recipes gives id of 0
    let new_id = match local_recipes.iter().max_by_key(|r| r.id){
        Some(v) => v.id +1,
        None => 0,
    };
    //pushes new recipe to local recipe content
    //to_owned used to transfer ownership
    local_recipes.push(Recipe{
        id: new_id,
        name: name.to_owned(),
        ingredients: ingredients.to_owned(),
        instructions: instructions.to_owned(),
        public: false
    });

    write_local_recipes(&local_recipes).await?;
    //feedback to the user
    info!("Created Recipe:");
    info!("Name: {:?}", name);
    info!("Ingredients: {:?}",ingredients);
    info!("Instructions: {:?}",instructions);
    //
    Ok(())
}
//logic for handling recipe publication
async fn handle_publish_recipes(cmd :&str){
    //removes the command from the string
    if let Some(rest) = cmd.strip_prefix("publish r"){
        //match statement check validity of id
        match rest.trim().parse::<usize>() {
            //
            Ok(id) =>{
                //
                if let Err(e) = publish_recipe(id).await{
                    info!("error publishing recipe with id {}, {}", id, e);
                }
                else {
                    info!("Successful publication with id {}",id);
                }
            }
            //if id doesnt match it spits out error
            Err(e)=> panic!("Invalid id {}, {}",rest.trim(),e),
        }
    }
}
//logic for publishing a recipe
async fn publish_recipe(id: usize)->Result<()>{
    let mut local_recipes = read_local_recipes().await?;
    //iterates through recipes and sets public flag to be true as the user intends to share it on the network
    local_recipes.iter_mut().filter(|r| r.id == id).for_each(|r| r.public = true);
    write_local_recipes(&local_recipes).await?;
    //
    Ok(())
}
//logic for reading local recipes
async fn read_local_recipes()-> Result<Recipes>{
    //reads content from storage
    let content = fs::read(STORAGE_FILE_PATH).await?;
    //deserialized result
    let result = serde_json::from_slice(&content)?;
    Ok(result)
}
//logic for writing local recipes
async fn write_local_recipes(recipes: &Recipes)->Result<()>{
    //Converts json to plain text
    let json = serde_json::to_string(&recipes)?;
    //Writes to local json file
    fs::write(STORAGE_FILE_PATH, &json).await?;
    //Ends function
    Ok(())
}
//logic for handling incoming recipe lists shared by other people
async fn handle_list_recipes(cmd :&str,swarm: &mut Swarm<RecipeBehaviour>){
    //Strips the command prefix as this isn't needed any more and its easier to parse the command without it
    let rest = cmd.strip_prefix("ls r");
    // Control flow to execute the correct code based off user command
    match rest {
        //If "all" command is encountered
        Some("all") => {
            let req = ListRequest {
                mode: ListMode::ALL,
            };
            //serializes to json
            let json = serde_json::to_string(&req).expect("can jsonify request");
            //publish it to previously mentioned topic
            swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
        //If peer id command is encountered
        Some(recipes_peer_id) => {
            let req = ListRequest {
                //
                mode: ListMode::One(recipes_peer_id.to_owned()),
            };
            //serializes to json
            let json = serde_json::to_string(&req).expect("can jsonify request");
            //publishes it to previously mentioned topic
            swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), json.as_bytes());
        }
        //if there is no command
        None => {
            //match statement catches error if no local recipes are present
            match read_local_recipes().await {
                //Ok(v) is the situation where there are local recipes
                Ok(v) => {
                    //outputs how many units there are in the local recipe list
                    info!("Local recipes ({})",v.len());
                    //iterates and outputs all local recipes to the user
                    v.iter().for_each(|r| info!("{:?}",r))
                }
                //out puts error if no recipes are found locally
                Err(e) => error!("error fetching local recipes: {}",e),
            }
        }
    }
}

//logic for responding incoming recipe requests by other people
fn respond_with_public_recipes(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
    //spawns new asynchronous task
    tokio::spawn(async move {
        //check if there are even any recipes to respond with
        match read_local_recipes().await {
            //case if recipe.json contains recipes
            Ok(recipes) => {
                //creates a response variable
                let resp = ListResponse {
                    mode: ListMode::ALL,
                    receiver,
                    //iterates through all recipes adding then to the data section
                    data: recipes.into_iter().filter(|r| r.public).collect(),
                };
                //"if let Err(e) specifies what to do if the message doesnt send
                if let Err(e) = sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            //error case
            Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
        }
    });
}





