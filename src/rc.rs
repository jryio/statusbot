use data_encoding::BASE64URL;
use hyper::{body::HttpBody, http::request::Builder, Body, Method, Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::env;
use time::OffsetDateTime;
use url::Url;

use crate::{bot::Status, secret::Secret, HttpsClient, Result};

// An example response length multiplied by 8 produces a maximum length of 2.2Mb response
const RC_SITE: &str = "RC_SITE";
const RC_BOT_ID: &str = "RC_BOT_ID";
const RC_APP_ID: &str = "RC_APP_ID";
const RC_APP_SECRET: &str = "RC_APP_SECRET";
const MAX_RESPONSE_BYES: u64 = 284701 * 8;
const AUTHORIZATION: &str = "Authorization";
const BASE_URL: &str = "rctogether.com";
const API_DESKS: &str = "/api/desks";
const API_BOTS: &str = "/api/bots";

#[derive(Debug)]
/// Recurse Client makes API requets to Virtual RC
pub struct RecurseClient {
    /// The base url of the Virtual RC instance.
    ///
    /// url = <subdomain>.rctogether.com
    pub url: Url,
    /// Bot ID is the Bot's ID. A Virtual RC application can spawn any number of bots, so this ID
    /// may change over time.
    ///
    /// For the time being, Status Bot's ID is '150139'
    pub bot_id: String,
    /// The HTTP client used for making outgoing HTTP requests
    client: HttpsClient,
    /// APP ID is the Virtual RC authorized appliation ID.
    ///
    /// This is used as the Username in HTTP Basic Auth
    app_id: Secret,
    /// Secret is the Vritual RC application's secret
    ///
    /// This is used as the Password in HTTP Basic Auth
    secret: Secret,
}

impl RecurseClient {
    /// Constructs a new RecurseClient instance configured to connect to <subdomain>.rctogether.com
    ///
    /// It uses the given APP_ID and SECRET as HTTP Basic Auth Username:Password
    ///
    /// It controls the bot with the given BOT_ID
    pub fn new(client: HttpsClient) -> Self {
        let app_id: Secret = env::var(RC_APP_ID)
            .expect("The .env file is missing RC_APP_ID")
            .into();
        let secret: Secret = env::var(RC_APP_SECRET)
            .expect("The .env file is missing RC_APP_SECRET")
            .into();
        let bot_id = env::var(RC_BOT_ID).expect("The .env file is missing RC_BOT_ID");
        let site = env::var(RC_SITE).expect("The .env file is missing RC_SITE");
        let url = Url::parse(&site).expect("The env variable RC_SITE is not a valid URL");
        Self {
            url,
            bot_id,
            client,
            app_id,
            secret,
        }
    }

    /// Constructs an http::request::Builder with method, uri, and HTTP Basic Auth.
    /// The caller must provide .body() and thus consume the RequestBuilder
    fn create_request(&self, method: Method, endpoint: &str) -> Builder {
        let credentials = format!(
            "{username}:{password}",
            username = self.app_id.to_string(),
            password = self.secret.to_string(),
        );
        let basic = format!("Basic {}", BASE64URL.encode(credentials.as_bytes()));
        Request::builder()
            .method(method)
            .uri(self.url.join(endpoint).unwrap().to_string())
            .header(
                // https://datatracker.ietf.org/doc/html/rfc7617
                AUTHORIZATION,
                basic,
            )
    }

    /// Given the Response returns the deserialized type T from the JSON body after performing
    /// content-length checks to prevent buffer overflows
    async fn read_json_body<T>(response: Response<Body>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response_content_length = match response.size_hint().upper() {
            Some(v) => v,
            None => MAX_RESPONSE_BYES + 1,
        };
        if response_content_length >= MAX_RESPONSE_BYES {
            return Err(
                format!("Recieved more than {MAX_RESPONSE_BYES} bytes in this response",).into(),
            );
        }
        let bytes = hyper::body::to_bytes(response.into_body()).await?;
        let body_str = String::from_utf8(bytes.to_vec())?;
        let data = serde_json::from_str::<T>(&body_str)?;
        return Ok(data);
    }

    /* -------------------------------------------------------------------------- */
    /*                                   API CALLS                                */
    /* -------------------------------------------------------------------------- */

    /// GET /api/desks
    ///
    /// Fetch all desks in Virtual RC
    pub async fn get_desks(&self) -> Result<GetDesksResponse> {
        let req = self
            .create_request(Method::GET, API_DESKS)
            .body(Body::empty())?;
        let res = self.client.request(req).await?;
        let desks: GetDesksResponse = Self::read_json_body(res).await?;
        Ok(desks)
    }

    pub async fn get_desk(&self, desk_id: usize) -> Result<Desk> {
        let desks = self.get_desks().await?;
        let mut desks = desks.0.iter();
        match desks.find(|d| d.id == desk_id) {
            Some(desk) => Ok(desk.clone()),
            None => Err(format!("Did not find a Virtual RC desk with id = {desk_id}").into()),
        }
    }

    /// PATCH /api/desks/:id
    ///
    /// Update the fields of a desk. Can be used to clear a desk's status by passing an empty [`Status`]
    pub async fn update_desk(&self, desk_id: u32, status: Status) -> Result<UpdateDeskResponse> {
        let endpoint = format!("{}/{}", API_DESKS, desk_id);
        let status_json = serde_json::to_string(&status)?;
        let req = self
            .create_request(Method::PUT, &endpoint)
            .body(Body::from(status_json))?;
        let res = self.client.request(req).await?;
        let updated_desk: UpdateDeskResponse = Self::read_json_body(res).await?;
        Ok(updated_desk)
    }

    /// PATCH /api/bots/:id
    ///
    /// This can upate the bot's properies, location, etc.
    pub async fn update_bot(&self, update_bot: UpdateBotRequest) -> Result<UpdateBotResponse> {
        let body = json!({
            "bot": update_bot,
        });
        let req = self
            .create_request(Method::PATCH, &format!("{}/{}", API_BOTS, self.bot_id))
            .body(body.to_string().into())?;

        let res = self.client.request(req).await?;
        debug!("Recurse Client -> update_bot -> response = {res:?}");
        // TODO: Need to handle what happens with the coordinates given are occupied.
        // The response in this case is
        // HTTP 422 - "{ "pos": [ "must not be in a block" ] }"
        // Should perform a directional search around the position until we find an open slotrc
        let updated_bot: UpdateBotResponse = Self::read_json_body(res).await?;
        Ok(updated_bot)
    }
}

/* -------------------------------------------------------------------------- */
/*                                  Data Types                                */
/* -------------------------------------------------------------------------- */
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct App {
    name: String,
    id: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EntityType {
    /// A person using RC Together
    Avatar,
    /// Similar to an avatar, but controlled by an app rather than a person
    Bot,
    /// A plain block that can be gray or colored, and can be labeled with a single letter
    Wall,
    /// A block where bots and avatars can leave notes for others to read.
    Note,
    /// A block that can contain a hyperlink.
    Link,
    /// A block that belongs to an avatar, and where its owner can set their status
    Desk,
    /// A block that lets users join a zoom meeting, and shows who's in it
    ZoomLink,
    /// Represents someone in a Zoom meeting who couldn't be matched to an Avatar
    UnknownAvatar,
    /// An area of the world where users are placed in a shared audio call.
    AudioRoom,
    /// A block used to describe and configure audio rooms
    AudioBlock,
    /// A block that shows events from the RC Calendar
    #[serde(rename = "RC::Calendar")]
    RcCalendar,
}

/// A string representing where a bot can be facing
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The Position of an entity in Virtual RC
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}
/// An Avatar is the circular representation of a person using Virtual RC
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Avatar {
    pub id: usize,
    pub name: String,
    pub image_url: String,
}

/// Desk
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Desk {
    /// The Desk ID
    pub id: usize,
    /// The Entity type, in this case "Desk"
    pub r#type: EntityType,
    /// The x, y coodinates for this desk
    pub pos: Position,
    /// The color of the desk, always "light-orange" cannot be modified
    pub color: String,
    /// The Unicode character representing the desk
    pub emoji: Option<String>,
    /// The text status of the text
    pub status: Option<String>,
    /// When the status (text and emoji) should expire.
    /// Specified as an ISO8601 formatted string.
    /// Required if you specify status.
    /// Cannot be more than 24 hours in the future.
    #[serde(default)]
    #[serde(with = "time::serde::iso8601::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// The Recurse Directory profile URL for the associated owner of this desk
    pub profile_url: Option<String>,
    /// Information on the onwer of this desk, including their ID, name, and image URL
    pub owner: Option<Avatar>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Bot {
    pub display_name: String,
    pub emoji: String,
    pub direction: String,
    pub can_be_mentioned: bool,
    pub app: App,
    /* messge: Option<Message> is the last message sent by this bot. Since this bot does not send
     * messages, this field shoudl always be null */
}

/* -------------------------------------------------------------------------- */
/*                                  Requests                                  */
/* -------------------------------------------------------------------------- */

/// A request body for PATCH /api/bots/:id
///
/// When making the request, all fields will be children of the "bot" field in the JSON
#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UpdateBotRequest {
    pub name: Option<String>,
    pub emoji: Option<String>,
    pub x: Option<usize>,
    pub y: Option<usize>,
    pub direction: Option<Direction>,
    pub can_be_mentioned: Option<bool>,
}

/* -------------------------------------------------------------------------- */
/*                                 Responses                                  */
/* -------------------------------------------------------------------------- */

/// A response from GET /api/desks
///
/// The response is a top-level JSON array with no fields,
/// so we use serde's #transparent container attribute to skip over
/// the single field in this struct an successfully deserialize it as
/// a top-level JSON array.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct GetDesksResponse(pub Vec<Desk>);

/// A response from PATCH /api/bots/:id
///
/// The response is equivalent to the Bot entity type,
/// so we wrap it and use #serde(transparent)
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UpdateBotResponse(pub Bot);

/// A response from PATCH /api/desks/:id
///
/// If the desk is updated successfully then the reponse is the
/// complete desk object with the updated fields applied.
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UpdateDeskResponse(pub Desk);
