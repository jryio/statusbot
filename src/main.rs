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

use std::sync::Arc;

// -----------------
// Imports
// -----------------
use crate::{bot::Bot, zulip::OutgoingWebhook};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Client, Method, Request, Response, Server, StatusCode,
};
use hyper_tls::HttpsConnector;

// -----------------
// Types
// -----------------
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type HttpsClient = Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

// -----------------
// Constants
// -----------------
// FIX: Use a properly configured evironment variable for this
// FIX: Use dotenf for this `use dotenv::dotenv`;
const ADDRESS: &str = "127.0.0.1:8080";
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
    // debug!("Deserialized outgoing-webhook: {webhook:#?}");

    // let bot_reply = bot.cmd_help(); // returns nothing
    // The incoming zulip message
    info!("message from user = {}", &webhook.data);

    let reply = bot.respond(webhook).await;

    // --> Response to Zulip
    let response = Response::builder().status(StatusCode::OK);
    let reply_json = serde_json::to_string(&reply)?;
    let response = response.body(reply_json.into())?;

    println!("response for POST /status {response:?}");

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Read ENV variables
    // match dotenv() {
    //     Ok(_path) => info!("Loaded .env file successfully"),
    //     Err(e) => error!("Failed to load .evn file with error = {e:?}"),
    // }

    // Configure server
    let address = ADDRESS.parse().unwrap();
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let bot = Arc::new(Bot::new(client.clone()));

    // Define HTTP service
    let http_service = make_service_fn(move |_| {
        // Hyper creates a new closure will be created for every incoming connection.
        // Additionally, once a connection is established, there may be multiple HTTP requests.
        let client = client.clone();
        let bot = bot.clone();

        #[allow(clippy::let_and_return)]
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        let service = async {
            Ok::<_, GenericError>(service_fn(move |req| {
                // Handle requests here
                let bot = bot.clone();
                handlers(req, bot)
            }))
        };
        service
    });

    info!("Started server on address: {ADDRESS}");
    let server = Server::bind(&address).serve(http_service);

    match server.await {
        Ok(_) => info!("Server exited successfully"),
        Err(e) => error!("Server error: {e:?}"),
    };

    Ok(())
}
