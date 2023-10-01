# This Dockerfile was modified from https://github.com/fly-apps/hello-rust

# -----------------
# --> Builder Image
# -----------------
FROM rust:latest as builder

# Sets the working directory of the container on the host machine
WORKDIR /usr/src/app

# Copies everything from the local machine to the image
COPY . .

# Will build and cache the binary and dependent crates in release mode
RUN --mount=type=cache, \
  target=/usr/local/cargo, \
  from=rust:latest, \
  source=/usr/local/cargo \
  --mount=type=cache, \
  target=target \
  cargo build --release && mv ./target/release/statusbot ./statusbot

# -----------------
# --> Runtime Image
# -----------------
FROM debian:bullseye-slim

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/statusbot /app/statusbot

# Run the app
CMD ./statusbot
