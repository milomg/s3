# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------

FROM rust:latest as cargo-build

RUN apt-get update

RUN apt-get install musl-tools -y

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app/s3

COPY Cargo.toml Cargo.toml

RUN mkdir src/

RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

RUN rm -f target/x86_64-unknown-linux-musl/release/deps/s3*

COPY ./src ./src

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl


# ------------------------------------------------------------------------------
# NPM Build Stage
# ------------------------------------------------------------------------------

FROM node:13 as npm-build

WORKDIR /app/client

COPY ./client/package*.json ./

RUN npm install

COPY ./client ./

RUN npm run build


# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

FROM alpine:latest

WORKDIR /app/bin/

COPY --from=cargo-build /app/s3/target/x86_64-unknown-linux-musl/release/s3 .

RUN mkdir -p client/dist/

COPY --from=npm-build /app/client/dist ./client/dist

CMD ["./s3"]