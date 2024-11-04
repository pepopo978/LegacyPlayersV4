FROM rust:1.68

RUN rustup install nightly-2023-01-22
RUN rustup default nightly-2023-01-22

RUN rustup target add wasm32-unknown-unknown

RUN mkdir /Backend
COPY ./src /Backend/src
COPY ./sub_crates /Backend/sub_crates
COPY ./Cargo.toml /Backend/Cargo.toml

WORKDIR /Backend

RUN export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse;cargo update
RUN export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse;cargo build --release

ENTRYPOINT ["/bin/bash", "-c", "export RUST_BACKTRACE=full;sleep 5s && /Backend/target/release/backend"]


