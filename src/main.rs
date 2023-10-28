// FIX: Remove this crate-level lint disable
#![allow(dead_code)]

mod bot;
mod rc;
mod secret;
mod zulip;

extern crate dotenv;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Client, Method, Request, Response, Server, StatusCode,
};
use hyper_tls::HttpsConnector;

use crate::{bot::Bot, zulip::OutgoingWebhook};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type HttpsClient = Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

// FIX: Use a properly configured evironment variable for this
// FIX: Use dotenf for this `use dotenv::dotenv`;
static ADDRESS: &str = "127.0.0.1:8080";
static NOTFOUND: &str = "NOT FOUND";
const STATUS_ENDPOINT: &str = "/status";

/// Match the incoming HTTP request Method and URI and call the corresponding handler
///
/// Each handler should be async (meaning it returns a Future)
async fn handlers(req: Request<Body>, client: HttpsClient) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        // POST /status
        (&Method::POST, STATUS_ENDPOINT) => handle_post_status(req, &client).await,
        // Return a basic 404 status code and text body for all other endpoints
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()),
    }
}

/// Handle an outgoing webhook (from Zulip to us) when Status Bot is mentioned in a chat.
///
/// Becuase we need to reply to Zulip's API, we will need access to a hyper::Client
///
/// The incomoing request will be formatting according to Zulip's 'Outgoing Webhooks'
/// documentation:
///
/// https://recurse.zulipchat.com/api/outgoing-webhooks#outgoing-webhook-format
async fn handle_post_status(req: Request<Body>, client: &HttpsClient) -> Result<Response<Body>> {
    debug!("incoming req for POST /status");

    // --> Receive outgoing webhook from Zulip
    let body = hyper::body::to_bytes(req.into_body()).await?;
    let body_string = String::from_utf8(body.to_vec())?;
    let webhook: OutgoingWebhook = serde_json::from_str(&body_string)?;
    // TODO: Only log if we fail to deserailize outgoing webhook from Zulip
    // debug!("Deserialized outgoing-webhook: {webhook:#?}");

    // --> Call out Bot struct / class and get some resposne from it
    let bot = Bot::new(client.clone());
    // let bot_reply = bot.cmd_help(); // returns nothing
    let sender_id = webhook.message.sender_id;
    // The message
    let data = webhook.data;
    let bot_reply = bot.cmd_status(sender_id, data).await?;

    // --> Response to Zulip
    // " { "content", """} "
    let response = Response::builder().status(StatusCode::OK);
    let response = response.body(bot_reply.into())?;

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

    // Define HTTP service
    // TODO: Setup shared state: Client, Bot Instance, etc.
    let http_service = make_service_fn(move |_| {
        // Hyper creates a new closure will be created for every incoming connection.
        // Additionally, once a connection is established, there may be multiple HTTP requests.

        let client = client.clone();

        #[allow(clippy::let_and_return)]
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        let service = async {
            Ok::<_, GenericError>(service_fn(move |req| handlers(req, client.to_owned())))
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
