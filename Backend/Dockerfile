FROM rust:1.68

RUN rustup install nightly-2023-01-22
RUN rustup default nightly-2023-01-22

RUN rustup target add wasm32-unknown-unknown

# Install heaptrack
RUN apt-get update && apt-get install -y heaptrack heaptrack-gui

RUN mkdir /Backend
COPY ./src /Backend/src
COPY ./sub_crates /Backend/sub_crates
COPY ./Cargo.toml /Backend/Cargo.toml
COPY ./Cargo.lock /Backend/Cargo.lock

WORKDIR /Backend

RUN cargo build --release

# Run with heaptrack
ENTRYPOINT ["/bin/bash", "-c", "export RUST_BACKTRACE=full; sleep 5s && if [ \"$USE_HEAPTRACK\" = \"true\" ] || [ \"$USE_HEAPTRACK\" = \"1\" ]; then heaptrack /Backend/target/release/backend; else /Backend/target/release/backend; fi"]