FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
RUN adduser --disabled-password --home /home/container container
USER container
ENV  USER=container HOME=/home/container
WORKDIR /home/container
COPY --from=builder /app/target/release/lurk_chan /bin/lurk_chan
COPY docker_entrypoint.sh /docker_entrypoint.sh
CMD ["/bin/bash","/docker_entrypoint.sh"]