mod zulip;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use hyper::{
    client::HttpConnector,
    service::{make_service_fn, service_fn},
    Body, Client, Method, Request, Response, Server, StatusCode,
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

// FIX: Use a properly configured evironment variable for this
static ADDRESS: &str = "127.0.0.1:8080";
static NOTFOUND: &str = "NOT FOUND";
const STATUS_ENDPOINT: &str = "/status";

/// Match the incoming HTTP request Method and URI and call the corresponding handler
///
/// Each handler should be async (meaning it returns a Future)
async fn handlers(req: Request<Body>, client: Client<HttpConnector>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        // POST /status
        //
        // The incomoing request will be formatting according to Zulip's 'Outgoing Webhooks'
        // documentation:
        // https://recurse.zulipchat.com/api/outgoing-webhooks#outgoing-webhook-format
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
async fn handle_post_status(
    req: Request<Body>,
    client: &Client<HttpConnector>,
) -> Result<Response<Body>> {
    debug!("incoming req for POST /status");
    let body = hyper::body::to_bytes(req.into_body()).await?;

    // should be JSON :)
    let body_string = String::from_utf8(body.to_vec())?;

    let webhook: std::result::Result<zulip::OutgoingWebhook, serde_json::Error> =
        serde_json::from_str(&body_string);

    debug!("Deserialized outgoing-webhook: {webhook:?}");

    let response = Response::builder().status(StatusCode::OK);

    let json_message = serde_json::json!({ "message": "it worked yo"});
    let json_string = json_message.to_string();
    let response = response.body(json_string.into())?;

    println!("response for POST /status {response:?}");

    Ok(response)
}
#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let address = ADDRESS.parse().unwrap();

    let client = Client::new();

    let http_service = make_service_fn(move |_| {
        let client = client.clone();
        async { Ok::<_, GenericError>(service_fn(move |req| handlers(req, client.to_owned()))) }
    });

    let server = Server::bind(&address).serve(http_service);
    info!("Started server on address: {ADDRESS}");

    server.await?;

    Ok(())
}
