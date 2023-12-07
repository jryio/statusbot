# This Dockerfile was modified from https://github.com/fly-apps/hello-rust

# -----------------
# --> Builder Image
# -----------------
FROM rust:bookworm as builder

# Sets the working directory of the container on the host machine
WORKDIR /usr/src/app

# Copies everything from the local machine to the image
COPY . .

ENV LANG en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8


# Will build and cache the binary and dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
  --mount=type=cache,target=target \
  cargo build --release && mv ./target/release/statusbot ./statusbot

# -----------------
# --> Runtime Image
# -----------------
FROM debian:bookworm-slim

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt install -y openssl ca-certificates locales locales-all

# Run as "app" user
RUN useradd -ms /bin/bash app

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
  dpkg-reconfigure --frontend=noninteractive locales && \
  update-locale LANG=en_US.UTF-8

RUN locale-gen en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US:en
ENV LC_ALL en_US.UTF-8

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/src/app/statusbot /app/statusbot

CMD ./statusbot
