// FIX: Remove this crate-level lint disable
#![allow(dead_code)]

// -----------------
// Crate Modules
// -----------------
mod bot;
mod rc;
mod secret;
mod zulip;

// -----------------
// External Crates
// -----------------
extern crate dotenv;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

// -----------------
// Imports
// -----------------
use crate::{
    bot::Bot,
    zulip::{OutgoingWebhook, ZulipEmoji},
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Client, Method, Request, Response, Server, StatusCode,
};
use hyper_tls::HttpsConnector;
use std::{net::SocketAddr, sync::Arc};

// -----------------
// Types
// -----------------
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type HttpsClient = Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

// -----------------
// Constants
// -----------------
const ENV_DEVEL: &str = ".env.devel";
const ENV_PROD: &str = ".env.prod";
const RUN_MODE: &str = "RUN_MODE";
const PROD: &str = "PROD";
const DEVEL: &str = "DEVEL";
const SERVER_DOMAIN: &str = "SERVER_DOMAIN";
const SERVER_PORT: &str = "SERVER_PORT";
const NOTFOUND: &str = "NOT FOUND";
const ROOT: &str = "/";
const STATUS_ENDPOINT: &str = "/status";

/// Match the incoming HTTP request Method and URI and call the corresponding handler
///
/// Each handler should be async (meaning it returns a Future)
async fn handlers(req: Request<Body>, bot: Arc<Bot>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, STATUS_ENDPOINT) => handle_post_status(req, bot).await,
        (&Method::GET, ROOT) => handle_get_root(req, bot).await,
        // Return a basic 404 status code and text body for all other endpoints
        // TODO: Made this a better 404
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()),
    }
}

/// Handle an incoming GET request to status bot
///
/// This functions as a heartbeat
async fn handle_get_root(_req: Request<Body>, _bot: Arc<Bot>) -> Result<Response<Body>> {
    Ok(Response::new(Body::from("Hello World!")))
}

/// Handle an outgoing webhook (from Zulip to us) when Status Bot is mentioned in a chat.
///
/// Becuase we need to reply to Zulip's API, we will need access to a hyper::Client
///
/// The incomoing request will be formatting according to Zulip's 'Outgoing Webhooks'
/// documentation:
///
/// https://recurse.zulipchat.com/api/outgoing-webhooks#outgoing-webhook-format
async fn handle_post_status(req: Request<Body>, bot: Arc<Bot>) -> Result<Response<Body>> {
    debug!("incoming req for POST /status");

    // --> Receive outgoing webhook from Zulip
    let body = hyper::body::to_bytes(req.into_body()).await?;
    let body_string = String::from_utf8(body.to_vec())?;
    let webhook: OutgoingWebhook = serde_json::from_str(&body_string)?;

    // TODO: Only log if we fail to deserailize outgoing webhook from Zulip
    debug!("Deserialized incoming-outgoing-webhook: {webhook:#?}");

    // let bot_reply = bot.cmd_help(); // returns nothing
    // The incoming zulip message
    info!("message from user = {}", &webhook.data);

    let reply = bot.respond(webhook).await;
    debug!("Main -> bot.respond(webhook) -> Reply = {:?}", reply);

    // --> Response to Zulip
    let response = Response::builder().status(StatusCode::OK);
    let reply_json = serde_json::to_string(&reply)?;
    let response = response.body(reply_json.into())?;

    println!("response for POST /status {response:?}");

    Ok(response)
}

/// Loads the correct .env file based on the the RUN_MODE envrionment variable
///
/// If RUN_MODE is set to PROD, the .env.prod is loaded
///
/// If RUN_MODE is set to DEVEL, then .env.devel is loaded
///
/// For everything else we default to loading .env.devel
fn load_env() {
    let env_file = match std::env::var(RUN_MODE) {
        Ok(val) => match val.as_str() {
            PROD => ENV_PROD,
            DEVEL => ENV_DEVEL,
            _ => ENV_DEVEL,
        },
        Err(_) => ENV_DEVEL,
    };
    match dotenv::from_filename(env_file) {
        Ok(_path) => info!("Loaded {env_file} file successfully"),
        Err(e) => error!("Failed to load {env_file} file with error = {e:?}"),
    };
}

fn make_address() -> SocketAddr {
    let server_domain =
        std::env::var(SERVER_DOMAIN).expect("The .env file is missing SERVER_DOMAIN");
    let server_port = std::env::var(SERVER_PORT).expect("The .env file is missing SERVER_PORT");
    format!("{server_domain}:{server_port}")
        .parse()
        .expect("SERVER_DOMAIN:SERVER_PORT is an invalid SocketAddr")
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    load_env();

    let address = make_address();
    let https = HttpsConnector::new();

    // Shared State
    let file = include_str!("zulip.json");
    let emoji: ZulipEmoji = serde_json::from_str(file).unwrap();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let mut bot = Bot::new(client.clone(), emoji);

    // TODO: Move this to a tokio::spawn background task that never gets canceled
    // https://stackoverflow.com/questions/66863385/how-can-i-use-tokio-to-trigger-a-function-every-period-or-interval-in-seconds
    let _res = bot.cache_desk_owners().await;
    let bot = Arc::new(bot);

    // Define HTTP Service
    let http_service = make_service_fn(move |_| {
        // Hyper creates a new closure will be created for every incoming connection.
        // Additionally, once a connection is established, there may be multiple HTTP requests.
        let bot = bot.clone();

        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        #[allow(clippy::let_and_return)]
        let service = async {
            Ok::<_, GenericError>(service_fn(move |req| {
                // Handle requests here
                let bot = bot.clone();
                handlers(req, bot)
            }))
        };
        service
    });

    info!("Started server on address: {address}");
    let server = Server::bind(&address).serve(http_service);

    match server.await {
        Ok(_) => info!("Server exited successfully"),
        Err(e) => error!("Server exited with error: {e:?}"),
    };

    Ok(())
}
