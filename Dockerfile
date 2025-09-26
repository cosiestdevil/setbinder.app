FROM rust:1-alpine3.20 AS builder
WORKDIR /usr/src/setbinder.app
COPY . .
RUN apk add pkgconfig openssl-dev libc-dev openssl-libs-static
RUN cargo install --path rocket-app

FROM alpine:3
RUN apk add openssl ca-certificates
COPY --from=builder /usr/local/cargo/bin/setbinder /usr/local/bin/setbinder.app
RUN mkdir -p /usr/local/templates/setbinder.app
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8080
ENV ROCKET_TEMPLATE_DIR=/usr/local/templates/setbinder.app
CMD ["setbinder.app"]