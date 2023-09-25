### Recurse Center - Status Bot

This is the repository for Status Bot, a helpful [Zulip bot](https://recurse.zulipchat.com/api/writing-bots) who update both your [Virtual
RC](https://rctogether.com) and [Zulip](https://zulip.com/) statuses during your batch.

This bot is written using the [Rust](https://rust-lang.org) programming language
and was primarily an effort to restore status-ing order to the world as well as
to learn Rust.


### Zulip API

Because this bot will not be written in Python, we sadly will not be able to use
the oh-so-nice [Zulip Python bot
library](https://recurse.zulipchat.com/api/writing-bots). Instead we can use
Zulip's [ Outgoing Webhooks
](https://recurse.zulipchat.com/api/outgoing-webhooks) to receive notifications
when Status Bot has been mentioned. Parsing and processing this JSON will be
simple enough in Rust.


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
