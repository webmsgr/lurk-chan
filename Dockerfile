FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --target x86_64-unknown-linux-musl --release

# We do not need the Rust toolchain to run the binary!
FROM alpine:latest AS runtime
RUN adduser --disabled-password --home /home/container container
USER container
ENV  USER=container HOME=/home/container
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info
ENV ENTRYPOINT=lurk_chan
WORKDIR /home/container
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/lurk_chan /bin/lurk_chan
COPY docker_entrypoint.sh /docker_entrypoint.sh
CMD ["/bin/sh","/docker_entrypoint.sh"]