FROM rust:1-alpine3.20 AS chef
# Install tools before copying source so application edits don't invalidate this layer.
RUN apk add --no-cache pkgconfig openssl-dev libc-dev openssl-libs-static \
    && cargo install cargo-chef --version 0.1.71 --locked
WORKDIR /usr/src/setbinder.app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY models/ models/
COPY scryfall/ scryfall/
COPY provider/ provider/
COPY archidekt-provider/ archidekt-provider/
COPY moxfield-provider/ moxfield-provider/
COPY rocket-app/ rocket-app/
RUN cargo chef prepare --recipe-path recipe.json

# Cloud Build publishes this target separately so compiled dependencies survive
# between build workers. No cache mounts are needed: target/ is part of the layer.
FROM chef AS dependencies
COPY --from=planner /usr/src/setbinder.app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json -p rocket-app --bin setbinder

FROM dependencies AS builder
COPY Cargo.toml Cargo.lock ./
COPY models/ models/
COPY scryfall/ scryfall/
COPY provider/ provider/
COPY archidekt-provider/ archidekt-provider/
COPY moxfield-provider/ moxfield-provider/
COPY rocket-app/ rocket-app/
RUN cargo build --release --locked -p rocket-app --bin setbinder

FROM alpine:3
RUN apk add --no-cache openssl ca-certificates \
    && mkdir -p /usr/local/templates/setbinder.app
COPY --from=builder /usr/src/setbinder.app/target/release/setbinder /usr/local/bin/setbinder.app
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8080
ENV ROCKET_TEMPLATE_DIR=/usr/local/templates/setbinder.app
CMD ["setbinder.app"]
