use crate::{secret::Secret, HttpsClient, Result};
use emojic::emojis::Emoji;
use hyper::{Method, Request};
use serde::{Deserialize, Serialize};

static HELP_TEXT: &str = r#"**How to use Status Bot**
* `status {emoji} {status text} {expiratoin time}`
* ``
* ``
* ``
* ``
* ``
* ``
* ``
"#;

/// A Status is constructed by parsing the message text sent to Status Bot in a direct message
///
/// At the time of writing the following different formats are supported :
///
/// * Setting a status with only text:
///     * `status Working on my project`
///     * `status In the hub`
///
/// * Setting a status with an emoji.
///     * *Note emojis are set via Zulip but cannot be custom emojis set in the Zulip organization.
///     Emoji aliases (:apple:) must be a default unicode emoji*.
///     * Alternatively, as long as the emoji is sent as a unicode character, then it will be
///     parsed succesfully. Only the first emoji will be parsed in the message string, all others
///     will be ignored.
///     * `status :smile: Excited for presentations!`
///     * `status :crab: Learning Rust today`
///     * `status :pear: Open to pairing`
///
/// * Setting a status with an expiration time:
///     * `status :bento box: Let's get lunch soon. All aboard the lunch train! <time:2023-09-29T12:00:00-06:00>`
pub struct Status {
    /// An emoji for the status.
    /// Default: 💻
    /// The default is only used if you specify status but not emoji.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    pub emoji: Option<Emoji>,

    /// The text status.
    /// The desk must have an owner in order to have a status.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    pub text: String,

    /// When the status (text and emoji) should expire.
    /// Specified as an ISO8601 timestamp.
    /// Required if you specify status.
    /// (?) Cannot be more than 24 hours in the future.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    // TODO: Examine whether the statement 'Cannot be more than 24 hours in the future' is correct
    // in the excerpt below from Virtual RC API.
    pub expires_at: Option<time::OffsetDateTime>,
}

/// Reply represents the Bot's response message to Zulip's outgoing webhook.
///
/// Responses to Zulip should be JSON formatted and can take two different options
///
/// Response Not Reqired:
///
/// ```
/// { "response_not_required": true }
/// ```
///
/// Content: (Zulip formatting markdown)
///
///  ```
/// { "content": "Status set successfully" }
/// ```
///
/// ```
/// { "content": "Failed to set status because ..." }
/// ```
///
///  ```
/// { "content": "**How to use Status Bot:** ..." }
/// ```
#[derive(Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum Reply {
    ResponseNotRequired { response_not_required: bool },
    Content { content: String },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct StatusResponse {
    code: Option<String>,
    msg: String,
    result: String,
}

pub struct Bot {
    client: HttpsClient,
    email: Secret,
    api_key: Secret,
    api_token: Secret,
    site: Secret,
}

impl Bot {
    pub fn new(client: HttpsClient) -> Bot {
        Bot {
            client,
            email: Secret("".into()),
            api_key: Secret("".into()),
            api_token: Secret("".into()),
            site: Secret("".into()),
        }
    }

    /// `status` - Sets the given status on both Virtual RC and Zulip
    pub async fn cmd_status(self, user_id: u64, status: String) -> Result<String> {
        // 3. call zulip api PUT /status
        //
        // 4. success? / fail?
        //
        // - if there is a failure respond
        // - when something fails email JACOB (oh nooo)
        //
        // 5. reply JSON

        let status_body = serde_json::json!({
            "status_text": status.clone(),
        });

        let set_status_req = Request::builder()
            .method(Method::POST)
            // .uri("https://recurse.zulipchat.com/api/v1/users/125378/status")
            .uri("https://recurse.zulipchat.com/api/v1/users/me/status")
            .header(
                "Authorization",
                format!(
                    "Basic {}:{}",
                    "status1-bot@zulipchat.com", "p7f6iCcvKrAVqmUjmXdIP7JueP2FMpeT"
                ),
            )
            .body(status_body.to_string().into())?;
        info!("Zulip status req: {:+?}", set_status_req);

        let resp = self.client.request(set_status_req).await;
        if let Ok(resp) = resp {
            info!("Zulip status resp: {:+?}", &resp);
            let body = hyper::body::to_bytes(resp.into_body()).await?;
            let body_string = String::from_utf8(body.to_vec())?;
            let status_resp: StatusResponse = serde_json::from_str(body_string.as_str())?;
            info!("\n\nZulip status resp body: {:+?}", status_resp);

            if status_resp.result == "error" && status_resp.code.is_some() {
                return Ok(
                    r#"{"content": "Failed to set the status, Zulip not happy with us :(" }"#
                        .into(),
                );
            }

            return Ok(r#"{"content": "Status set!" }"#.into());
        } else {
            error!("Zulip status resp: {:+?}", resp);
            Ok("".into())
        }
    }

    /// `clear` - Unsets the currents status on Virtual RC and Zulip
    pub fn cmd_clear(self) {
        unimplemented!()
    }

    /// `help` - Responds to the user with a help message detailing the different comands and configurations
    /// they can run using StatusBot
    pub fn cmd_help(self) -> String {
        return r#"{ "content": "How to use Status Bot: Send me 'status' to set status" }"#.into();
    }

    /// `feedback` - Writes feedback to the bot authors
    pub fn cmd_feedback(self) {
        unimplemented!()
    }
}
