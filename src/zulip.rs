/*
{
    "bot_email": "outgoing-bot@localhost",
    "bot_full_name": "Outgoing webhook test",
    "data": "@**Outgoing webhook test** Zulip is the world\u2019s most productive group chat!",
    "message": {
        "avatar_url": "https://secure.gravatar.com/avatar/1f4f1575bf002ae562fea8fc4b861b09?d=identicon&version=1",
        "client": "website",
        "content": "@**Outgoing webhook test** Zulip is the world\u2019s most productive group chat!",
        "display_recipient": "Verona",
        "id": 112,
        "is_me_message": false,
        "reactions": [],
        "recipient_id": 20,
        "rendered_content": "<p><span class=\"user-mention\" data-user-id=\"25\">@Outgoing webhook test</span> Zulip is the world\u2019s most productive group chat!</p>",
        "sender_email": "iago@zulip.com",
        "sender_full_name": "Iago",
        "sender_id": 5,
        "sender_realm_str": "zulip",
        "stream_id": 5,
        "subject": "Verona2",
        "submessages": [],
        "timestamp": 1527876931,
        "topic_links": [],
        "type": "stream"
    },
    "token": "xvOzfurIutdRRVLzpXrIIHXJvNfaJLJ0",
    "trigger": "mention"
}
*/

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// In Zulip 8.0 this was renamed to 'direct_message' from 'private_message'
    DirectMessage,
    Mention,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    #[serde(rename(deserialize = "text/html"))]
    Html,
    #[serde(rename(deserialize = "text/x-markdown"))]
    Markdown,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DisplayRecipient {
    Recipients(Vec<User>),
    Stream(String),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct User {
    email: String,
    full_name: String,
    id: u64,
    is_mirror_dummy: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Message {
    /// The URL of the message sender's avatar. Can be null only if the current user has access
    /// to the sender's real email address and client_gravatar was true.
    ///
    /// If null, then the sender has not uploaded an avatar in Zulip, and the client can compute the
    /// gravatar URL by hashing the sender's email address, which corresponds in this case to their
    /// real email address.
    pub avatar_url: Option<String>,
    /// A Zulip "client" string, describing what Zulip client sent the message.
    pub client: String,
    /// The content/body of the message.
    pub content: String,
    /// The HTTP content_type for the message content. This will be text/html or text/x-markdown,
    /// depending on whether apply_markdown was set.
    pub content_type: ContentType,
    /// Data on the recipient of the message; either the name of a stream or a dictionary
    /// containing basic data on the users who received the message.
    pub display_recipient: DisplayRecipient,
    /// Note we skip deserializing edit_history for this bot
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct OutgoingWebhook {
    /// Email of the bot user
    pub bot_email: String,
    /// The full name of the bot user
    pub bot_full_name: String,
    /// The message content, in raw Markdown format (not rendered to HTML)
    pub data: String,
    /// What aspect of the message triggered the outgoing webhook notification. Possible values
    /// include direct_message and mention.
    pub trigger: Trigger,
    /// A string of alphanumeric characters that can be used to authenticate the webhook request
    /// (each bot user uses a fixed token). You can get the token used by a given outgoing webhook
    /// bot in the zuliprc file downloaded when creating the bot.
    pub token: String,
    /// A dictionary containing details on the message that triggered the outgoing webhook, in the
    /// format used by GET /messages.
    pub message: Message,
}
