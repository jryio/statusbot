use std::str::SplitWhitespace;

use crate::{
    rc::RecurseClient,
    secret::Secret,
    zulip::{OutgoingWebhook, Trigger},
    HttpsClient, Result,
};
use emojic::emojis::Emoji;
use regex::Regex;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, OffsetDateTime};

const SPACE: &str = " ";
const RE_EMOJI: &str = "(?<emoji>:.+?:)?";
const RE_TEXT: &str = "(?<text>[^<>\r\n\t]+)?";
const RE_TIME: &str = "(<time:(?<iso8061>.+?)>)?";
const HELP_TEXT: &str = r#"**How to use Status Bot**:
* `status {emoji}? {status} {expiration}?` Set your status
    * `{emoji}` optional. Emoji must be the unicode character for the emoji, not its short name like :apple:
    * `{status}` required. Status can be any length string
    * `{expiration}` optional. Expiration should be set using zulip's <time> selector
* `show` Display your current
* `clear` Clears your status
* `feedback {text}` Provide feedback to the Status Bot maintainer(s). Currently this is **@Jacob Young**
* `help` Prints this help message

Have you found a bug with Status Bot? Please [create an issue on Github](https://github.com/jryio/statusbot/issues/new)
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
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// An emoji for the status.
    /// Default: 💻
    /// The default is only used if you specify status but not emoji.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    pub emoji: Option<&'static Emoji>,

    /// The text status.
    /// The desk must have an owner in order to have a status.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    pub text: Option<String>,

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

/// A Command Status Bot knows about
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Status(Status),
    Show,
    Clear,
    Feedback(String),
    Help,
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
#[derive(Serialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub enum Reply {
    ResponseNotRequired { response_not_required: bool },
    Content { content: String },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct ZulipStatusResponse {
    code: Option<String>,
    msg: String,
    result: String,
}

pub struct Bot {
    /// A Hyper HttpsClient
    client: HttpsClient,
    /// An instance of a Virtual RC HTTP Client
    rc: RecurseClient,
    /// The Bot's email used as a username for Zulip API requests
    /// E.g. status1-bot@zulipchat.com
    email: Secret,
    /// The Zulip Bot's api key
    api_key: Secret,
    /// A string of alphanumeric characters that can be used to authenticate the webhook request
    /// (each bot user uses a fixed token). You can get the token used by a given outgoing webhook
    /// bot in the zuliprc file downloaded when creating the bot.
    api_token: Secret,
    /// The URL of the Zulip Instance
    /// E.g. https://<subdoamin>.zulipchat.com
    site: Secret,
}

impl Bot {
    /// Creates a new Status Bot instance
    pub fn new(client: HttpsClient) -> Bot {
        // TODO: Instantiate a Zulip Client and store it (note: it will be without the user's API Key)

        let rc = RecurseClient::new(client.clone());
        Bot {
            client,
            rc,
            email: Secret("".into()),
            api_key: Secret("".into()),
            api_token: Secret("".into()),
            site: Secret("".into()),
        }
    }

    /// Repond will parse the incoming message to Status Bot, determine which command was invoked,
    /// and call the appropriate command.
    ///
    /// If the user did not send a valid bot command, it will reply with the help text
    ///
    /// All responses shoudl be valid Zulip Messsage Formatting
    pub async fn respond(&self, webhook: OutgoingWebhook) -> Reply {
        if webhook.trigger == Trigger::Mention {
            return Reply::ResponseNotRequired {
                response_not_required: true,
            };
        }

        if webhook.token != self.api_token {
            info!(
                "Invalid bot token.\
                Recieved an incoming webhook for a different bot?\
                bot_email={} bot_full_name={}",
                webhook.bot_email, webhook.bot_full_name
            )
        }

        let message = webhook.data;
        let user_id = webhook.message.sender_id;
        let command = Self::parse_cmd(&message);
        let result = self.run_command(command, user_id);

        // TODO: Handle the result of the match. If any cmd methods returned a result, something
        // went wrong and we should reply to the user with a message saying "Status Bot was unable
        // to perform <cmd> because of <reason>. If you believe status bot is not working, please
        // write a Zulip message to <maintianers>"
        result.unwrap()
    }
    fn run_command(&self, command: Command, user_id: u64) -> Result<Reply> {
        match command {
            Command::Help => self.cmd_help(),
            Command::Show => self.cmd_show(),
            Command::Clear => self.cmd_clear(),
            Command::Feedback(f) => self.cmd_feedback(&f),
            Command::Status(status) => self.cmd_status(user_id, status),
        }
    }

    /// Parses the input message from the user into one of the known Status Bot commands.
    /// If no command can be parsed, the help command is run showing help text
    fn parse_cmd(message: &str) -> Command {
        // If the message is empty or entirely whitespace, the iterator will yield None
        // split_whitespace() will also handle \t \n and other unicode whitespaces
        let mut splits = message.split_whitespace();
        if let Some(first) = splits.next() {
            return match first {
                "help" => Command::Help,
                "show" => Command::Show,
                "clear" => Command::Clear,
                "feedback" => {
                    let feedback = Self::parse_feedback(splits);
                    match feedback.len() {
                        0 => Command::Help,
                        _ => Command::Feedback(feedback),
                    }
                }
                "status" => {
                    let mut peakable = splits.by_ref().peekable();
                    // peek() returns a &T so if T = &str we get &&str
                    let first = peakable.peek();
                    match first {
                        Some(_) => {
                            let rest = Self::fold_splits(splits);
                            let status = Self::parse_status(rest);
                            todo!()
                        }
                        // We only got `status` with no arguments
                        None => Command::Help,
                    }
                }
                // Any other words
                _ => Command::Help,
            };
        };

        // Empty mesage, entirely whitespace
        Command::Help
    }

    /// Parse feedback collects all of the remaining words back into a string
    fn parse_feedback(splits: SplitWhitespace<'_>) -> String {
        Self::fold_splits(splits)
    }

    /// Parse status handles the different valid combinations to construct a  [`Status`]
    ///
    /// CASE : status :emoji:
    /// CASE : status :emoji: hi
    /// CASE : status :emoji: hi <time:XXXX>
    /// CASE : status :emoji: <time:YYYY>
    fn parse_status(input: String) -> Option<Status> {
        let mut status = Status::default();
        let re_status = format!("{}{}{}", RE_EMOJI, RE_TEXT, RE_TIME);
        let re_status = Regex::new(&re_status).unwrap();

        return match re_status.captures(&input) {
            Some(caps) => {
                if let Some(emoji) = caps.name("emoji") {
                    status.emoji = emojic::parse_alias(emoji.as_str());
                }

                if let Some(text) = caps.name("text") {
                    status.text = Some(text.as_str().into())
                }

                if let Some(time) = caps.name("time") {}

                return Some(status);
            }
            None => None,
        };

        // 1- Is the next word an emoji? (Starts with :, followed by some words in snake_case, then
        // ends with :)
        //      * If it is, use emojic to convert it into a unicode character, set statsus.emoji
        //      equal to it
        //      * If it is an emoji alias but not a standard name, then return an error [`Reply`] to the user
        // 2 - Look at the last item in the SplitWhitespace, is it a <time:*> string?
        //      * If it is, use a some parsing logic to get the time out in UTC?
        //      * If it is, but an invalid time, return an error reply to the user
        // 3 - Collect the remaining characters as the status text
        let mut status = Status::default();
        let mut peekable = splits.by_ref().peekable();
        let first = *peekable.peek().unwrap(); /* We know we have at least this value because of parse_cmd::status */

        // The first item is an emoji
        if let Some(emoji) = emojic::parse_alias(first) {
            // Set the emoji status and progress the iterator
            status.emoji = Some(*emoji);
            peekable.next();
        }

        // Pop the last item and check if it is a <time:---> formatted string
        if let Some(last) = peekable.next_back() {
            // If it's not a time, we should try to add it back to the iterator
            let re_time = Regex::new(r"<time:(?<iso8061>.+)>").unwrap();
            if let Some(caps) = re_time.captures(last) {
                let maybe_iso8061 = caps.name("iso8061").map_or("", |m| m.as_str());
                // Attempt to parse it into a valid OffsetDateTime
                if let Ok(date_time) = OffsetDateTime::parse(maybe_iso8061, &Iso8601::DEFAULT) {
                    status.expires_at = Some(date_time);
                }
            }
            // Build the status now that we have emoji and/or time
            // Set aside a string for the status.text
            let mut text = String::default();
            let mut words = peekable.collect::<Vec<&str>>();
            // No expire_at time was set on the Status
            // so we should add the last item back on to the iterator
            if status.expires_at.is_none() {
                words.push(last);
            }
            text = words.join(SPACE);
            if text.len() > 0 {
                status.text = Some(text);
            }
        }
        // If we hit this then we only got `status :emoji:` and nothing as the last
        // Having a status with no text is valid
        else {
        }

        status
    }

    fn fold_splits(splits: SplitWhitespace) -> String {
        splits
            .fold(String::new(), |mut a, b| {
                a.reserve(b.len() + 1);
                a.push_str(b);
                a.push_str(" ");
                a
            })
            .trim_end()
            .into()
    }

    /// `status` - Sets the given status on both Virtual RC and Zulip
    pub fn cmd_status(&self, user_id: u64, status: Status) -> Result<Reply> {
        todo!();
        // let status_body = serde_json::json!({
        //     "status_text": status.clone(),
        // });

        // let set_status_req = Request::builder()
        //     .method(Method::POST)
        //     // .uri("https://recurse.zulipchat.com/api/v1/users/125378/status")
        //     .uri("https://recurse.zulipchat.com/api/v1/users/me/status")
        //     .header(
        //         "Authorization",
        //         format!(
        //             "Basic {}:{}",
        //             "status1-bot@zulipchat.com", "p7f6iCcvKrAVqmUjmXdIP7JueP2FMpeT"
        //         ),
        //     )
        //     .body(status_body.to_string().into())?;
        // info!("Zulip status req: {:+?}", set_status_req);

        // let resp = self.client.request(set_status_req).await;
        // if let Ok(resp) = resp {
        //     info!("Zulip status resp: {:+?}", &resp);
        //     let body = hyper::body::to_bytes(resp.into_body()).await?;
        //     let body_string = String::from_utf8(body.to_vec())?;
        //     let status_resp: ZulipStatusResponse = serde_json::from_str(body_string.as_str())?;
        //     info!("\n\nZulip status resp body: {:+?}", status_resp);

        //     if status_resp.result == "error" && status_resp.code.is_some() {
        //         return Ok(
        //             r#"{"content": "Failed to set the status, Zulip not happy with us :(" }"#
        //                 .into(),
        //         );
        //     }

        //     return Ok(r#"{"content": "Status set!" }"#.into());
        // } else {
        //     error!("Zulip status resp: {:+?}", resp);
        //     Ok("".into())
        // }
    }

    /// `show` - Displays the user's current status on Virtual RC
    pub fn cmd_show(&self) -> Result<Reply> {
        // TODO: Fetch the status of the current user
        todo!()
    }

    /// `clear` - Unsets the currents status on Virtual RC and Zulip
    pub fn cmd_clear(&self) -> Result<Reply> {
        // TODO: Clears the status associated with the current user
        todo!()
    }

    /// `help` - Responds to the user with a help message detailing the different comands and configurations
    /// they can run using StatusBot
    pub fn cmd_help(&self) -> Result<Reply> {
        Ok(Reply::Content {
            content: HELP_TEXT.into(),
        })
    }

    /// `feedback` - Writes feedback to the bot authors
    pub fn cmd_feedback(&self, feedback: &str) -> Result<Reply> {
        // TODO: Send a Zulip message with the feedback to the list of maintainers configured in .env
        todo!()
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
#[cfg(test)]
mod tests {
    use super::Bot;
    use super::Command;

    #[test]
    fn test_commmand_splitting() {
        let input = "help".into();
        let output = Bot::parse_cmd(input);
        assert_eq!(output, Command::Help);

        let input = "show".into();
        let output = Bot::parse_cmd(input);
        assert_eq!(output, Command::Show);

        let input = "feedback".into();
        let output = Bot::parse_cmd(input);
        assert_eq!(output, Command::Help);

        let input = "feedback This thing is great!".into();
        let output = Bot::parse_cmd(input);
        assert_eq!(output, Command::Feedback("This thing is great!".into()));

        let input = "clear".into();
        let output = Bot::parse_cmd(input);
        assert_eq!(output, Command::Clear);

        // TODO: Test Command::Status(Status)
    }

    #[test]
    fn test_parse_status() {}
}
