use data_encoding::BASE64URL;
use hyper::{body::HttpBody, http::request::Builder, Body, Method, Request, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::env;
use time::OffsetDateTime;
use url::Url;

use crate::{bot::Status, consts::*, secret::Secret, HttpsClient, Result};

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
            .header("Content-Type", "application/json")
    }

    /// Given a position in the grid, attempt
    fn surrounding_positions(&self, pos: &Position) -> Vec<Position> {
        // Top
        let top = Position {
            x: pos.x,
            y: (pos.y - 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // Top Left
        let top_left = Position {
            x: (pos.x - 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: (pos.y - 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // Top Right
        let top_right = Position {
            x: (pos.x + 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: (pos.y - 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // Left
        let left = Position {
            x: (pos.x - 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: pos.y,
        };
        // Right
        let right = Position {
            x: (pos.x + 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: pos.y,
        };
        // Bottom
        let bottom = Position {
            x: pos.x,
            y: (pos.y + 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // Bottom Left
        let bottom_left = Position {
            x: (pos.x - 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: (pos.y + 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // Bottom Right
        let bottom_right = Position {
            x: (pos.x + 1).clamp(GRID_X_MIN, GRID_X_MAX),
            y: (pos.y + 1).clamp(GRID_Y_MIN, GRID_Y_MAX),
        };
        // We put left and right at the start of the list of positions because of the common
        // orientation of RC desks in Virtual RC.
        vec![
            left,
            right,
            top,
            top_left,
            top_right,
            bottom,
            bottom_left,
            bottom_right,
        ]
    }

    /// Given the Response returns the deserialized type T from the JSON body after performing
    /// content-length checks to prevent buffer overflows
    async fn read_json_body<T>(response: Response<Body>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response_content_length = match response.body().size_hint().upper() {
            Some(v) => v,
            None => MAX_RESPONSE_BYES,
        };
        if response_content_length > MAX_RESPONSE_BYES {
            debug!(
                "RC -> read_json_body -> response_content_length = {:?}",
                response.size_hint().upper()
            );
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
    pub async fn update_desk(
        &self,
        desk_id: usize,
        desk_pos: &Position,
        status: Status,
    ) -> Result<Desk> {
        let endpoint = format!("{}/{}", API_DESKS, desk_id);
        // let status_json = serde_json::to_string(&status)?;
        let desk_json = json!({
            "bot_id": self.bot_id,
            "desk": status,
        })
        .to_string();
        let req_update_desk = self
            .create_request(Method::PATCH, &endpoint)
            .body(Body::from(desk_json))?;
        debug!("Bot -> update_desk -> request = {:#?}", req_update_desk);
        // Before upating a desk, we have to move the StatusBot instance to the correct location
        // next to the desk. So we try all the surrounding positions.
        for pos in self.surrounding_positions(desk_pos) {
            if let Ok(_) = self
                .update_bot(UpdateBotRequest {
                    name: None,
                    emoji: None,
                    x: Some(pos.x),
                    y: Some(pos.y),
                    direction: None,
                    can_be_mentioned: None,
                })
                .await
            {
                match self.client.request(req_update_desk).await {
                    Ok(res) => match res.status() {
                        StatusCode::OK => match Self::read_json_body::<Desk>(res).await {
                            Ok(updated_desk) => {
                                return Ok(updated_desk);
                            }
                            Err(e) => {
                                debug!("RC -> update_desk -> found surrounding pos -> update_desk -> deserialize error = {e}");
                                return Err(e);
                            }
                        },
                        StatusCode::UNPROCESSABLE_ENTITY => {
                            debug!("RC -> update_desk -> HTTP 422 -> Request was invalid");
                            return Err("The request had an invalid JSON body and the Virtual RC API rejected it".into());
                        }
                        StatusCode::BAD_REQUEST => {
                            return Err("The request was invalid in some way and the Vritual RC API rejected it".into());
                        }
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            return Err(
                                "The Virtual RC API fell over becuase of our request".into()
                            );
                        }
                        // All other status codes
                        status @ _ => {
                            return Err(format!(
                                "The Vritual RC API returned an unkown HTTP header value: {status}"
                            )
                            .into());
                        }
                    },
                    Err(e) => {
                        debug!("RC -> update_desk -> found surrounding pos -> update_desk call -> network error = {e}");
                        return Err(e.into());
                    }
                }
            }
        }
        Err(format!("Bot was unable to find an open grid position next to desk (id = {desk_id}, pos = {desk_pos:?})").into())
    }

    /// PATCH /api/desks/:id/cleanup
    ///
    /// This endpoint will clear the values of a desks's status, emoji, and expires_at
    pub async fn cleanup_desk(&self, desk_id: usize) -> Result<Desk> {
        // TODO: Unfortunately this endpoint removes the owner of the desk as well.
        // It can't be used until this behavior is patch on API side.
        let endpoint = format!("{}/{}/{}", API_DESKS, desk_id, DESKS_CLEANUP);
        let body_json = json!({
            "bot_id": self.bot_id,
        })
        .to_string();
        let req_cleanup_desk = self
            .create_request(Method::PATCH, &endpoint)
            .body(Body::from(body_json))?;
        match self.client.request(req_cleanup_desk).await {
            Ok(res) => Self::read_json_body::<Desk>(res).await.map_err(|err| {
                format!("Failed to deserialize response from {endpoint}. Err = {err}").into()
            }),
            Err(e) => Err(format!("Request to {endpoint} failed with error = {e}").into()),
        }
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
            .body(Body::from(body.to_string()))?;
        debug!("Bot -> update_bot -> request = {:#?}", req);

        let res = self.client.request(req).await?;
        let result = match res.status() {
            StatusCode::OK => {
                let updated_bot: UpdateBotResponse = Self::read_json_body(res).await?;
                // let updated_bot = UpdateBotResponse(Bot {
                //     id: 0,
                //     pos: Position { x: 0, y: 0 },
                //     r#type: "".into(),
                //     name: "".into(),
                //     message: None,
                //     display_name: "".into(),
                //     emoji: "".into(),
                //     can_be_mentioned: false,
                //     direction: "".into(),
                //     app: App {
                //         name: "".into(),
                //         id: 0,
                //     },
                // });
                Ok(updated_bot)
            }
            StatusCode::UNPROCESSABLE_ENTITY => Err("Must not be in a block".into()),
            _ => Err("Unknown error while trying to update bot".into()),
        };
        // let bytes = to_bytes(res.into_body()).await?;
        // let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        // debug!("Recurse Client -> update_bot -> response = {body_str:#?}");
        result
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
pub struct Message {
    pub text: String,
    pub sent_at: String,
    pub mentioned_agent_ids: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Bot {
    pub id: usize,
    pub r#type: String,
    pub name: String,
    pub display_name: String,
    pub emoji: String,
    pub direction: String,
    pub can_be_mentioned: bool,
    pub pos: Position,
    pub app: App,
    pub message: Option<Message>,
    /* messge: Option<Message> is the last message sent by this bot. Since this bot does not send
     * messages, this field should always be null */
}

/* -------------------------------------------------------------------------- */
/*                                  Requests                                  */
/* -------------------------------------------------------------------------- */

/// A request body for PATCH /api/bots/:id
///
/// When making the request, all fields will be children of the "bot" field in the JSON
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct UpdateBotRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
