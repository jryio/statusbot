/* Main */
pub const ENV_DEVEL: &str = ".env.devel";
pub const ENV_PROD: &str = ".env.prod";
pub const RUN_MODE: &str = "RUN_MODE";
pub const FLY_APP_NAME: &str = "FLY_APP_NAME";
pub const PROD: &str = "PROD";
pub const DEVEL: &str = "DEVEL";
pub const SERVER_DOMAIN: &str = "SERVER_DOMAIN";
pub const SERVER_PORT: &str = "SERVER_PORT";
pub const DESKS_INTERVAL: u64 = 1 * 60; /* 1 minutes */
pub const NOTFOUND: &str = "NOT FOUND";
pub const ROOT: &str = "/";
pub const STATUS_ENDPOINT: &str = "/status";

/* RC */
pub const RC_SITE: &str = "RC_SITE";
pub const RC_BOT_ID: &str = "RC_BOT_ID";
pub const RC_APP_ID: &str = "RC_APP_ID";
pub const RC_APP_SECRET: &str = "RC_APP_SECRET";

pub const GRID_X_MAX: usize = 169;
pub const GRID_X_MIN: usize = 0;
pub const GRID_Y_MAX: usize = 109;
pub const GRID_Y_MIN: usize = 0;

pub const MAX_RESPONSE_BYES: u64 = 284701 * 16;
pub const AUTHORIZATION: &str = "Authorization";

pub const API_DESKS: &str = "/api/desks";
pub const API_BOTS: &str = "/api/bots";

/* Bot */
pub const ZULIP_BOT_EMAIL: &str = "ZULIP_BOT_EMAIL";
pub const ZULIP_BOT_API_KEY: &str = "ZULIP_BOT_API_KEY";
pub const ZULIP_BOT_API_TOKEN: &str = "ZULIP_BOT_API_TOKEN";
pub const ZULIP_SITE: &str = "ZULIP_SITE";
pub const BOT_HOME_X: &str = "RC_BOT_HOME_X";
pub const BOT_HOME_Y: &str = "RC_BOT_HOME_Y";

pub const SPACE: &str = " ";
pub const COMMA: &str = ",";

// This regex will match (pronouns)?(batch)? but not the name section
pub const RE_RC_PRONOUNS: &str = r"(?<pronouns>\([a-z/]+\))?";
pub const RE_RC_BATCH: &str = r"(\((W|SP|Sp|S|F|m)\d?'\d{2}\))?";
pub const RE_RC_NAME_PARTS: &str = r".*?";

// Combined = (:(.+?):)?([^<>\r\n\t]+)?(<time:(.+?)>)?
pub const RE_EMOJI: &str = "(:(?<emoji>.+?):)?";
pub const RE_STATUS: &str = "(?<status>[^<>\r\n\t]+)?";
pub const RE_TIME: &str = "(<time:(?<iso8061>.+?)>)?";

pub const EMPTY_STATUS: &str = r"Your status is empty";
pub const MISSING_DESK: &str = r#"**Unable to a find a desk in Vritual RC asssociated with your username**
* Make sure you have [claimed a desk](https://recurse.notion.site/RC-Together-User-Guide-695cc163c76c47449347bd97a6842c3b) in Virutal RC
* If your Zulip username does not match your Virtual RC username... (ignoring the (pronouns) and (batch) parentheticals)
  * Fix username mismatch between Zulip <-> Virtual RC by using command `set_name {name}` with Status Bot
  * `set_name {name}` Tell Status Bot your Virtual RC username
* If Status Bot still cannot find your desk in Virtual RC, please [create an issue](https://github.com/jryio/statusbot/issues/new) on Github
"#;
// Missing text:
// * `clear` Clear your status
// * `feedback {text}` Provide anonymous feedback to the Status Bot maintainer(s)
pub const HELP_TEXT: &str = r#"**How to use Status Bot**:
* `status {emoji} {text} {expires_at}` Set your status
  * `{emoji}` (optional) - A unicode emoji
    * Custom emojis are not supported (:sadparrot:)
  * `{status}` (optional) - Status message for others to see
    * Cannot contain `<` or `>` characters
  * `{expires_at}` optional - The expiration time for the status (default 30m)
    * Expiration should be set using zulip's [<time> selector](https://zulip.com/help/global-times)
    * Choose a time in the future!
  * `status :crab: Rewriting Status Bot in Rust <time:2025-01-01T10:00:00-04:00>`
* `show` Display your current status
* `help` Print help message

Note: Status Bot uses your Zulip username to match your Virtual RC username. If you're having
trouble setting your status you can tell Status Bot what your Virtual RC name is with the
command `set_name {name}`

Bug with Status Bot? Please [create an issue](https://github.com/jryio/statusbot/issues/new) on Github
"#;
