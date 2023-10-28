use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::secret::Secret;

const BASE_URL: &str = "rctogether.com";

/// Recurse Client makes API requets to Virtual RC
pub struct RecurseClinet {
    /// The sub-domain of the Virtual RC instance.
    ///
    /// <subdomain>.rctogether.com
    pub subdomain: String,
    /// Bot ID is the Bot's ID. A Virtual RC application can spawn any number of bots, so this ID
    /// may change over time.
    ///
    /// For the time being, Status Bot's ID is '150139'
    pub bot_id: String,
    /// APP ID is the Virtual RC authorized appliation ID.
    ///
    /// This is used as the Username in HTTP Basic Auth
    app_id: Secret,
    /// Secret is the Vritual RC application's secret
    ///
    /// This is used as the Password in HTTP Basic Auth
    secret: Secret,
}

impl RecurseClinet {
    /// Constructs a new RecruseClient instance configured to connect to <subdomain>.rctogether.com
    ///
    /// It uses the given APP_ID and SECRET as HTTP Basic Auth Username:Password
    ///
    /// It controls the bot with the given BOT_ID
    pub fn new(subdomain: String, app_id: Secret, secret: Secret, bot_id: String) -> Self {
        Self {
            subdomain,
            bot_id,
            app_id,
            secret,
        }
    }
    /// GET <subdomain>.rctogether.com/api/desks
    pub fn get_desks(&self) {
        unimplemented!()
    }

    /// PATCH <subdomain>.rctogether.com/api/bots/:id
    pub fn update_bot(&self) {
        unimplemented!()
    }
}

/* -------------------------------------------------------------------------- */
/*                                  Data Types                                */
/* -------------------------------------------------------------------------- */

#[derive(Serialize, Deserialize)]
pub enum EntityType {
    // A person using RC Together
    Avatar,
    // Similar to an avatar, but controlled by an app rather than a person
    Bot,
    // A plain block that can be gray or colored, and can be labeled with a single letter
    Wall,
    // A block where bots and avatars can leave notes for others to read.
    Note,
    // A block that can contain a hyperlink.
    Link,
    // A block that belongs to an avatar, and where its owner can set their status
    Desk,
    // A block that lets users join a zoom meeting, and shows who's in it
    ZoomLink,
    // Represents someone in a Zoom meeting who couldn't be matched to an Avatar
    UnknownAvatar,
    // An area of the world where users are placed in a shared audio call.
    AudioRoom,
    // A block used to describe and configure audio rooms
    AudioBlock,
    // A block that shows events from the RC Calendar
    #[serde(rename = "RC::Calendar")]
    RcCalendar,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// A string representing where a bot can be facing
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize)]
// The Position of an entity in Virtual RC
pub struct Position {
    x: usize,
    y: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Avatar {
    id: usize,
    name: String,
    image_url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Desk
pub struct Desk {
    pub id: usize,
    pub r#type: EntityType,
    pub pos: Position,
    pub color: String,
    pub emoji: Option<String>,
    pub status: Option<String>,
    /// When the status (text and emoji) should expire,
    /// specified as an ISO8601 timestamp.
    /// Required if you specify status.
    /// Cannot be more than 24 hours in the future.
    // TODO: Is it true that you cannot set the time more than 24 hours in the future?
    #[serde(default)]
    #[serde(with = "time::serde::timestamp::option")]
    pub expires_at: Option<OffsetDateTime>,
    /// The Recurse Directory profile URL for the associated owner of this desk
    pub profile_url: Option<String>,
    pub ownder: Option<Avatar>,
}

/* -------------------------------------------------------------------------- */
/*                                  Requests                                  */
/* -------------------------------------------------------------------------- */

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
/// A request body for PATCH /api/bots/:id
///
/// When making the request, all fields will be children of the "bot" field in the JSON
pub struct UpdateBotRequest {
    name: Option<String>,
    emoji: Option<String>,
    x: Option<usize>,
    y: Option<usize>,
    direction: Option<Direction>,
    can_be_mentioned: Option<bool>,
}

/* -------------------------------------------------------------------------- */
/*                                 Responses                                  */
/* -------------------------------------------------------------------------- */

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
/// A response from GET /api/desks
///
/// The response is a top-level JSON array with no fields,
/// so we use serde's #transparent container attribute to skip over
/// the single field in this struct an successfully deserialize it as
/// a top-level JSON array.
pub struct GetDesksRespnse {
    desks: Vec<Desk>,
}
