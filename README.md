### Recurse Center - Status Bot

This is the repository for Status Bot, a helpful [Zulip bot](https://recurse.zulipchat.com/api/writing-bots) who update both your [Virtual
RC](https://rctogether.com) and [Zulip](https://zulip.com/) statuses during your batch.

This bot is written using the [Rust](https://rust-lang.org) programming language
and was primarily an effort to restore status-ing order to the world as well as
to learn Rust.

### Installing

[Install Rust](https://www.rust-lang.org/learn/get-started) via `rustup`

```sh
# Latest version of rust 
rustup update 

# Download and build this crate and its dependencies defined in Cargo.toml
cargo build

```

### Running Locally

```sh
# Rust goes brrrr
RUST_LOG=trace cargo run

# For active local development, install cargo-watch
cargo install cargo-watch

RUST_LOG=trace cargo watch -c -w src -x run

```

**Testing Locally**

* With the webserver running locally we can send a `POST` request using Zulip's
  example outgoing webhook data stored in `webhook.json`

```sh
curl -X POST -H "Content-Type: application/json" -d @./webhook.json 127.0.0.1:8080/status
```

### Deploying w/ Docker & fly.io

**TODO**

#### Zulip API

Because this bot will not be written in Python, we sadly will not be able to use
the oh-so-nice [Zulip Python bot
library](https://recurse.zulipchat.com/api/writing-bots). Instead we can use
Zulip's [ Outgoing Webhooks
](https://recurse.zulipchat.com/api/outgoing-webhooks) to receive notifications
when Status Bot has been mentioned. Parsing and processing this JSON will be
simple enough in Rust.

#### Virtual RC API

[Virtual RC has an API](https://docs.rctogether.com/#introduction) for things like pet bots, maze bots, and all sorts of other things. We will be using it to update the status.

To authenticate with Virtual RC, we need to first [create an app] at [example.rctogether.com/apps] where we obtain:

1. An `app_id`
2. An `app_secret`

All API requests will be made using HTTP basic auth with `username`: `app_id`
and `password`: `app_secret`. Alternatively we can pass these as URL parameters.

In Virtual RC, each user may optionally have created a [desk](https://docs.rctogether.com/#desk-fields) for themselves.
Their desk may optionally contain a status which consists of:

1. A optional `status` string
2. An optional `emoji` of the current status
3. An optional `expires_at` time at which the current status is removed


Virtual RC bots (which use the API) may update their desk's status by using
[`PATCH api/desks/:id`](https://docs.rctogether.com/#update-a-desk). The bot may
only update a desk which belongs to the user or is unclaimed.


#### Rust HTTP and JSON

Here are some common and possible choices for crates to allow HTTP
request/response and JSON communication with the Zulip API.

* [hyper](https://hyper.rs/) is a no-frills HTTP networking library written in
  Rust. It setup a basic HTTP server and handle requests
* [axum](https://github.com/tokio-rs/axum) is a more fully featured async HTTP
  library for Rust and the Tokio ecosystem. It provides many nice abstractions
  and traits over Request/Response using the [tower::Service](https://docs.rs/tower/latest/tower/trait.Service.html)
  trait. While this is an ergonomic choice, it may include to much "magic" for those wishing to learn
  from the beginning.
* [serde](https://serde.rs/) is an amazing Rust library allowing for generic
  serialization and deserialization of any Rust data type with helpful and easy
  to use macros and annotations. This will certainly be a crate used in this
  project.


#### Registration Flow


Status bot should work immediately for setting their Zulip status.


The following is required for connecting a Zulip account to virtual RC:

1. User direct messages @Status Bot and sends `desk {desk_id}`. This is the
   user's `desk_id` in Virtual RC. Sending status bot `help` should give
   instructions on how to find their `desk_id`.


2. Status bot records `zulip_user_id -> virtual_rc_desk_id` (file or otherwise).

3. All future direct messages to @Status Bot with `status "{user's status}"`
   should set both Zulip status and Virtual RC status.



#### Open Questions

* Is it possible for the user to send status bot a time in which they would like
  the status to expire?

  * `status ":octopus: working until 10am" 10am`
  * `status ":octopus: working until for 2 hours" 2hrs`
  * `status ":beach: Out of office until 10/20" 2w`
  * `status ":painting: "Presentation on thursday" thurs`
  * `status ":laptop: Joining book club at <time:2023-09-28T09:30:00-06:00>`

* If a time is set to expire, status bot will need to schedule a future API call
  to unset both Zulip and Virtual RC statuses.

  * What is the best way in rust to schedule an action to occur at a specific
    date and time?
