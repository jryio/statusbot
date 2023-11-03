use std::str::SplitWhitespace;
use std::{collections::HashMap, env};

use crate::{
    rc::RecurseClient,
    secret::Secret,
    zulip::{OutgoingWebhook, Trigger, ZulipEmoji},
    HttpsClient, Result,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, OffsetDateTime};

const ZULIP_BOT_EMAIL: &str = "ZULIP_BOT_EMAIL";
const ZULIP_BOT_API_KEY: &str = "ZULIP_BOT_API_KEY";
const ZULIP_BOT_API_TOKEN: &str = "ZULIP_BOT_API_TOKEN";
const ZULIP_SITE: &str = "ZULIP_SITE";

const SPACE: &str = " ";
const COMMA: &str = ",";

// This regex will match (pronouns)?(batch)? but not the name section
const RE_RC_PRONOUNS: &str = r"(?<pronouns>\([a-z/]+\))?";
const RE_RC_BATCH: &str = r"(\((W|SP|Sp|S|F|m)\d?'\d{2}\))?";
const RE_RC_NON_NAME_PARTS: &str = r".*?";

// Combined = (:(.+?):)?([^<>\r\n\t]+)?(<time:(.+?)>)?
const RE_EMOJI: &str = "(:(?<emoji>.+?):)?";
const RE_TEXT: &str = "(?<text>[^<>\r\n\t]+)?";
const RE_TIME: &str = "(<time:(?<iso8061>.+?)>)?";

const MISSING_DESK: &str = r#"**Unable to a find a desk in Vritual RC asssociated with your username**
* Make sure you have [claimed a desk](https://recurse.notion.site/RC-Together-User-Guide-695cc163c76c47449347bd97a6842c3b) in Virutal RC
* Make sure your Zulip username matches your Virtual RC username (ignore the (pronouns) and (batch) sections in your Zulip username)
  * Fix a Zulip <-> Virtual RC name mismatch by using command `set_name {name}` with Status Bot
  * `set_name {name}`
* If Status Bot still cannot find your desk in Virtual RC, please [create an issue](https://github.com/jryio/statusbot/issues/new) on Github
"#;
const HELP_TEXT: &str = r#"**How to use Status Bot**:
* `status {emoji} {text} {expires_at}` Set your status
  * `{emoji}` (optional) - A unicode emoji
    * Custom emojis are not supported (:sadparrot:)
  * `{status}` (optional) - Status message for others to see
    * Cannot contain `<` or `>` characters
  * `{expires_at}` optional - The expiration time for the status
    * Expiration should be set using zulip's [<time> selector](https://zulip.com/help/global-times)
    * Choose a time in the future!
  * E.g: `status :crab: Rewriting Status Bot in Rust <time:2025-01-01T10:00:00-04:00>`
* `show` Display your current status
* `clear` Clear your status
* `feedback {text}` Provide anonymous feedback to the Status Bot maintainer(s)
* `help` Print help message

Note: Status Bot uses your Zulip username to match your Virtual RC username. If you're having
trouble setting your status you can tell Status Bot what your Virtual RC name is with the
command `set_name {name}`

Bug with Status Bot? Please [create an issue](https://github.com/jryio/statusbot/issues/new) on Github
"#;

#[derive(Debug)]
pub struct Bot {
    /// A Hyper HttpsClient
    client: HttpsClient,
    /// A [`ZulipEmoji`] HashMap from zulip emoji alias to standard emoji alias
    emojis: ZulipEmoji,
    /// Virtual RC [Owner.Name] -> Desk_ID
    ///
    /// Zuliup usernames are used to looup in this table. Maybe not be a perfect match
    pub desk_owners: HashMap<String, usize>,
    /// Manually set the Virtual RC name associated with this Zulip username
    pub corrected_names: HashMap<String, String>,
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
    pub fn new(client: HttpsClient, emojis: ZulipEmoji) -> Bot {
        let rc = RecurseClient::new(client.clone());
        let desk_owners = HashMap::new();
        let corrected_names = HashMap::new();
        let email = env::var(ZULIP_BOT_EMAIL).expect("ZULIP_BOT_EMAIL is not set in the .env file");
        let api_key =
            env::var(ZULIP_BOT_API_KEY).expect("ZULIP_BOT_API_KEY is not set in the .env file");
        let api_token =
            env::var(ZULIP_BOT_API_TOKEN).expect("ZULIP_BOT_API_TOKEN is not set in the .env file");
        let site = env::var(ZULIP_SITE).expect("ZULIP_SITE is not set in the .env file");

        Bot {
            client,
            emojis,
            desk_owners,
            corrected_names,
            rc,
            email: Secret(email.into()),
            api_key: Secret(api_key.into()),
            api_token: Secret(api_token.into()),
            site: Secret(site.into()),
        }
    }

    /// Initialize a HashMap of user_id to desk_id
    ///
    /// This must be called before any other API request becuase we need a local cache of all
    /// vrc_user_id -> desk_id to match based on the Zulip sender in our incoming [`OutgoingWebhook`]
    pub async fn cache_desk_owners(&mut self) -> Result<()> {
        // Fetch all desks
        // Parse the response
        //  take the desk.owner.name field as the key
        //  take the desk.id as the value
        // Done
        let desks = self.rc.get_desks().await?;
        self.desk_owners = desks
            .0
            .iter()
            .filter(|desk| desk.owner.is_some())
            .map(|desk| {
                let owner = desk.owner.as_ref().unwrap();
                let desk_id = desk.id;
                let name = &owner.name;
                (name, desk_id)
            })
            .fold(HashMap::new(), |mut map, (name, desk_id)| {
                let _ = map.insert(name.clone(), desk_id);
                map
            });

        Ok(())
    }

    /// Repond will parse the incoming message to Status Bot, determine which command was invoked,
    /// and call the appropriate command, then send a Zulip reply
    ///
    /// If the user did not send a valid bot command, it will reply with the help text
    ///
    /// All responses should be valid Zulip Messsage Formatting
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
                 bot_email={} bot_full_name={} token={}",
                webhook.bot_email, webhook.bot_full_name, webhook.token,
            )
        }

        let message = webhook.data;
        let zuid = webhook.message.sender_id;
        let username = webhook.message.sender_full_name;
        let rc_username = self.lookup_desk_id(&username);
        if rc_username == None {
            debug!("bot -> respond -> lookup_desk_id returned None. Unable to find a desk for this zulip_username = {username}. Replied with MISSING_DESK text");
            return Reply::Content {
                content: MISSING_DESK.into(),
            };
        }
        // TODO: Make parse_cmd return an error, interpret the error string a the message body of a [`Reply`]
        let command = self.parse_cmd(&message);
        let result = self.run_command(command, zuid).await;

        // TODO: Handle the result of the match. If any cmd methods returned a result, something
        // went wrong and we should reply to the user with a message saying "Status Bot was unable
        // to perform <cmd> because of <reason>. If you believe status bot is not working, please
        // write a Zulip message to <maintianers>"
        result.unwrap()
    }

    /// Runs the function associated with the command
    async fn run_command(&self, command: Command, zuid: u64) -> Result<Reply> {
        match command {
            Command::Help => self.cmd_help().await,
            Command::Show => self.cmd_show(zuid).await,
            Command::Clear => self.cmd_clear(zuid).await,
            Command::Feedback(f) => self.cmd_feedback(&f).await,
            Command::SetName(n) => self.cmd_set_name(zuid, n).await,
            Command::Status(status) => self.cmd_status(zuid, status).await,
            // Testing Commands (hidden)
            Command::TestMissingDesk => self._cmd_test_missing_desk().await,
        }
    }

    /// `status` - Sets the given status on both Virtual RC and Zulip
    pub async fn cmd_status(&self, zuid: u64, status: Status) -> Result<Reply> {
        todo!();
    }

    /// `show` - Displays the user's current status on Virtual RC
    pub async fn cmd_show(&self, zuid: u64) -> Result<Reply> {
        //
        todo!()
    }

    /// `clear` - Unsets the currents status on Virtual RC and Zulip
    pub async fn cmd_clear(&self, zuid: u64) -> Result<Reply> {
        let empty = Status::default();
        // self.rc.update_desk(empty).await;
        todo!()
    }

    /// `help` - Responds to the user with a help message detailing the different comands and configurations
    /// they can run using StatusBot
    pub async fn cmd_help(&self) -> Result<Reply> {
        Ok(Reply::Content {
            content: HELP_TEXT.into(),
        })
    }

    /// `feedback` - Writes feedback to the bot authors
    pub async fn cmd_feedback(&self, feedback: &str) -> Result<Reply> {
        // TODO: Send a Zulip message with the feedback to the list of maintainers configured in .env
        todo!()
    }

    /// `set_name` - Updates the Zulip user's display name. Used resolving naming issues between
    /// Zulip and Virtual RC.
    /// TODO: Store this result somewhere other than memory. What options does fly.io give us?
    pub async fn cmd_set_name(&self, zuid: u64, new_name: String) -> Result<Reply> {
        todo!()
    }

    /// Testing Function to return MISSING_DESK help text
    pub async fn _cmd_test_missing_desk(&self) -> Result<Reply> {
        debug!("Command TestMissingDesk -> MISSING_DESK = {}", MISSING_DESK);
        Ok(Reply::Content {
            content: MISSING_DESK.into(),
        })
    }

    /// Looks up the associated desk for the zulip username. If this user provided a username
    /// correction then the corrected name will be used to lookup the desk_id instead.
    fn lookup_desk_id(&self, zulip_username: &str) -> Option<usize> {
        // 1 - Replace the zulip name with a virtual RC name if one was set in corrected_names
        // 2 - With the (new) virtual RC name, try to find an entry in desk_owners.
        // 3 - If one in found, return it
        // 4 - Otherwise, reply with an error (Reply)
        let re_non_name = Regex::new(
            format!("{}{}{}", RE_RC_NON_NAME_PARTS, RE_RC_PRONOUNS, RE_RC_BATCH).as_str(),
        )
        .unwrap();
        let zulip_username = re_non_name.replace_all(&zulip_username, "").to_string();
        let zulip_username = zulip_username.trim();
        debug!("lookup_desk_id -> replaced_zulip_username = '{zulip_username}'");

        let name = self
            .corrected_names
            .get(zulip_username)
            .map_or(zulip_username, |new_name| new_name.as_str());

        debug!("lookup_desk_id -> zulip_username = {zulip_username} name = {name}");
        self.desk_owners.get(name).map(|id| *id)
    }

    /// Parses the input message from the user into one of the known Status Bot commands.
    /// If no command can be parsed, the help command is run showing help text
    // TODO: Make parse_cmd return an error, interpret the error string a the message body of a [`Reply`]
    fn parse_cmd(&self, message: &str) -> Command {
        // If the message is empty or entirely whitespace, the iterator will yield None
        // split_whitespace() will also handle \t \n and other unicode whitespaces
        let mut splits = message.split_whitespace();
        if let Some(first) = splits.next() {
            debug!("Parse Command -> first = {first}");
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
                    // Check if we received any of the optional arguments for status
                    let input = Self::fold_splits(splits.clone());
                    let mut peekable = splits.by_ref().peekable();
                    let first = peekable.peek();
                    match first {
                        Some(_) => {
                            let status = self.parse_status(input);
                            Command::Status(status)
                        }
                        // We only got `status` with no arguments
                        None => Command::Help,
                    }
                }
                // Testing Commands (hidden)
                "test_missing_desk" => Command::TestMissingDesk,
                // Any other words
                _ => Command::Help,
            };
        };

        // Empty mesage, entirely whitespace
        Command::Help
    }

    /// Collects all of the remaining words back into a string
    fn parse_feedback(splits: SplitWhitespace<'_>) -> String {
        Self::fold_splits(splits)
    }

    /// Handles the different valid combinations to construct a  [`Status`]
    fn parse_status(&self, input: String) -> Status {
        let mut status = Status::default();
        let re_status = format!(r"{}\s?{}\s?{}", RE_EMOJI, RE_TEXT, RE_TIME);
        let re_status = Regex::new(&re_status).unwrap();

        return match re_status.captures(&input) {
            Some(caps) => {
                if let Some(maybe_alias) = caps.name("emoji") {
                    let maybe_alias = maybe_alias.as_str().trim();
                    debug!("maybe_alias = {maybe_alias}");
                    // We may get multiple aliases for this emoji (E.g. "first,second")
                    if let Some(aliases) = self.emojis.0.get(maybe_alias) {
                        debug!("Got aliases for {maybe_alias} = {aliases}");
                        // If there is one alias and no ',' we will only get one item
                        for alias in aliases.split(COMMA) {
                            debug!("one alias of {maybe_alias} = {alias}");
                            // If we already successfully set the Status' emoji from a previous
                            // alias, then we should not overwrite with None if the second alias is
                            // incorrect for some reason
                            if status.emoji.is_none() {
                                let emoji = emojic::parse_alias(alias);
                                debug!("status has no emoji so we're going to set it. parse_alias for {alias} = {emoji:?}");
                                status.emoji = emoji.map_or(None, |e| Some(e.grapheme.into()));
                            }
                        }
                    }
                    // This means we couldn't find a matching emoji alias. Either the user gave us a custom emoji, mispelled it, or we have an out of date zulip emoji.json
                    else {
                        debug!("failed to find alias = {maybe_alias} in out ZulipEmoji");
                    }
                }

                if let Some(text) = caps.name("text") {
                    let text = text.as_str().trim();
                    status.text = if text.len() > 0 {
                        Some(text.into())
                    } else {
                        None
                    };
                }

                // TODO: Don't allow expirations in the past!
                if let Some(maybe_iso8061) = caps.name("iso8061") {
                    let maybe_iso8061 = maybe_iso8061.as_str().trim();
                    if let Ok(date_time) = OffsetDateTime::parse(maybe_iso8061, &Iso8601::DEFAULT) {
                        status.expires_at = Some(date_time);
                    }
                }

                status
            }
            None => Status::default(),
        };
    }

    /// Collects a SplitsIterator into a String
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
}

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
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// An emoji for the status.
    /// Default: 💻
    /// The default is only used if you specify status but not emoji.
    ///
    /// Source: https://docs.rctogether.com/#update-a-desk
    pub emoji: Option<String>,

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
    SetName(String),
    Help,
    TestMissingDesk,
}

/// Reply represents the Bot's response message to Zulip's outgoing webhook.
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

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
#[cfg(test)]
mod tests {
    use std::sync::Once;

    use hyper::Client;
    use hyper_tls::HttpsConnector;
    use test_case::test_case;
    use time::macros::datetime;

    use crate::bot::Status;
    use crate::load_env;
    use crate::zulip::ZulipEmoji;

    use super::Bot;
    use super::Command;

    use once_cell::sync::OnceCell;
    static BOT: OnceCell<Bot> = OnceCell::new();
    static LOGGER: Once = Once::new();

    pub fn init() {
        LOGGER.call_once(|| {
            pretty_env_logger::init();
        });
        // Load the .env file based
        load_env();
        // Create a new Bot
        let file = include_str!("zulip.json");
        let emoji: ZulipEmoji = serde_json::from_str(file).unwrap();
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        let bot = Bot::new(client, emoji);
        // Intentionally do nothing with the error because
        // it's okay to attempt to set mulitple times
        let _ = BOT.set(bot);
    }

    pub fn get_test_bot() -> &'static Bot {
        BOT.get().expect("Bot was not initialized")
    }

    #[test_case("status" => Command::Help ; "test status command empty")]
    #[test_case("status :apple: watching the Apple keynote <time:2025-01-01T13:00:00-04:00>"
        => Command::Status(Status{
            emoji: Some(emojic::flat::RED_APPLE.grapheme.into()),
            text: Some("watching the Apple keynote".into()),
            expires_at: Some(datetime!(2025-01-01 13:00:00 -4))
        })
        ; "test status command full")]
    #[test_case("help" => Command::Help ; "test help command")]
    #[test_case("show" => Command::Show ; "test show command")]
    #[test_case("feedback" => Command::Help ; "test feedback empty gives help command")]
    #[test_case("feedback this bot sucks" => Command::Feedback("this bot sucks".into()) ; "test feedback")]
    #[test_case("clear" => Command::Clear ; "test clear command")]
    #[test_case("random" => Command::Help ; "test invalid command gives help command")]
    #[test_case("" => Command::Help ; "test empty input gives help command")]
    fn test_commmand_splitting(input: &str) -> Command {
        init();
        let bot = get_test_bot();
        bot.parse_cmd(input)
    }

    // Empty
    #[test_case("", "", "" => Status{ emoji: None, text: None, expires_at: None }
        ; "test empty status")]
    // Emoji Test Cases
    #[test_case(":apple:", "", "" => Status{ emoji: Some(emojic::flat::RED_APPLE.grapheme.into()), text: None, expires_at: None }
        ; "test single valid emoji apple")]
    #[test_case(":call_me:", "", "" => Status{ emoji: Some(emojic::flat::CALL_ME_HAND.grapheme.into()), text: None, expires_at: None }
        ; "test single valid emoji call me")]
    #[test_case(":flag_european_union:", "", "" => Status{ emoji: Some(emojic::flat::FLAG_EUROPEAN_UNION.grapheme.into()), text: None, expires_at: None }
        ; "test single valid emoji flag european union")]
    #[test_case(":this_shortcode_does_not_exist:", "", "" => Status{ emoji: None, text: None, expires_at: None }
        ; "test invalid emoji shortcode")]
    #[test_case("apple", "", "" => Status{ emoji: None, text: Some("apple".into()), expires_at: None }
        ; "test emoji name is not interpreted as emoji")]
    #[test_case("::", "", "" => Status{ emoji: None, text: Some("::".into()), expires_at: None }
        ; "test empty emoji shortcode")]
    // Text Test Cases
    #[test_case("hey I'm setting my status at RC successfully", "", "" =>
        Status{ emoji: None, text: Some("hey I'm setting my status at RC successfully".into()), expires_at: None }
        ; "test simple text status")]
    #[test_case("this - status . has \\ lots of punctuation?!+_#@%^)(!%^)", "", "" =>
        Status{ emoji: None, text: Some("this - status . has \\ lots of punctuation?!+_#@%^)(!%^)".into()), expires_at: None }
        ; "test status with lots of punctutation")]
    #[test_case("שמן ךן שגי שך ןשי", "", "" =>
        Status{ emoji: None, text: Some("שמן ךן שגי שך ןשי".into()), expires_at: None }
        ; "test status non-english language")]
    // Time Test Cases
    #[test_case("", "", "<time:2023-11-29T20:00:00-05:00>" =>
        Status{ emoji: None, text: None, expires_at: Some(datetime!(2023-11-29 20:00:00 -5)) }
        ; "test basic datetime")]
    #[test_case("", "", "<time:>" =>
        Status{ emoji: None, text: None, expires_at: None }
        ; "test invalid datetime")]
    #[test_case("", "", "<time:2023-11-29>" =>
        Status{ emoji: None, text: None, expires_at: None }
        ; "test ivalid only date")]
    #[test_case("", "", "<>" =>
        Status{ emoji: None, text: None, expires_at: None }
        ; "test no time keyword")]
    // Combinations
    #[test_case(":octopus:", "Octopass is the best checkin project!", "" =>
        Status{ emoji: Some(emojic::flat::OCTOPUS.grapheme.into()), text: Some("Octopass is the best checkin project!".into()), expires_at: None }
        ; "test emoji and text")]
    #[test_case(":octopus:", "", "<time:2023-11-29T20:00:00-05:00>" =>
        Status{ emoji: Some(emojic::flat::OCTOPUS.grapheme.into()), text: None, expires_at: Some(datetime!(2023-11-29 20:00:00 -5)) }
        ; "test emoji and time")]
    #[test_case(":octopus:", "Octopass is the best checkin project!", "<time:2023-11-29T20:00:00-05:00>" =>
        Status{ emoji: Some(emojic::flat::OCTOPUS.grapheme.into()), text: Some("Octopass is the best checkin project!".into()), expires_at: Some(datetime!(2023-11-29 20:00:00 -5)) }
        ; "test emoji and text and time")]
    #[test_case("", "Octopass is the best checkin project!", "<time:2023-11-29T20:00:00-05:00>" =>
        Status{ emoji: None, text: Some("Octopass is the best checkin project!".into()), expires_at: Some(datetime!(2023-11-29 20:00:00 -5)) }
        ; "test text and time")]
    fn test_statuses(emoji: &str, text: &str, expires_at: &str) -> Status {
        init();
        let bot = get_test_bot();
        let input = format!("{emoji} {text} {expires_at}");
        bot.parse_status(input.into())
    }
}
