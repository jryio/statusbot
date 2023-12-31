### Recurse Center - Status Bot

This is the repository for Status Bot, a helpful [Zulip bot](https://recurse.zulipchat.com/api/writing-bots) who update both your [Virtual
RC](https://rctogether.com) and [Zulip](https://zulip.com/) statuses during your batch.

This bot is written using the [Rust](https://rust-lang.org) programming language
and was primarily an effort to restore status-ing order to the world as well as
to learn Rust.

A large effort was made to make the source code easy to read, well
documented, tested, and maintainable.

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
RUST_LOG=trace cargo run

# For active local development, install cargo-watch
cargo install cargo-watch
RUST_LOG=trace cargo watch -c -w src -x run

```

### Deploying w/ Docker & fly.io

**TODO**

### Virtual RC API

[Virtual RC has an API](https://docs.rctogether.com/#introduction) for things like pet bots, maze bots, and all sorts of other things. We will be using it to update the status.

To authenticate with Virtual RC, we need to first [create an app] at [example.rctogether.com/apps] where we create:

1. An `app_id`
2. An `app_secret`

All API requests are made using HTTP basic auth with `username`: `app_id`
and `password`: `app_secret`. Alternatively we can pass these as URL parameters.

In Virtual RC, each user may optionally have created a [desk](https://docs.rctogether.com/#desk-fields) for themselves.
Their desk may optionally contain a status which consists of:

1. A optional `status` string
2. An optional `emoji` of the current status
3. An optional `expires_at` when the status should expire 


Virtual RC bots (which use the API) may update their desk's status by using
[`PATCH api/desks/:id`](https://docs.rctogether.com/#update-a-desk). The bot may
only update a desk which belongs to the user or is unclaimed.


### Rust HTTP and JSON

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


### Extracting Zulip Emoji Names

Status Bot needs to know which emoji aliases map to unicode characters for
emojis. For example: `:apple:` in Zulip maps to 🍎. However the official unicode
alias for 🍎 is `:red_apple:`.

Basically Zulip has done some realiasing and we have to invert the aliases so we
know unicode characters they are supposed to map to.

I forked the zulip repository and wrote a Python script to read through their
emoji aliases and output a JSON file which 

The forked branch can be found [here](https://github.com/zulip/zulip/compare/main...jryio:zulip:jryio/emoji-extraction)
