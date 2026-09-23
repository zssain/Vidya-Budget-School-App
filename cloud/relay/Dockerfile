# Vidya relay — multi-stage build → distroless (glibc, non-root). The relay links
# rustls(ring), NOT OpenSSL, so no system TLS libs are needed at runtime.
#
# Build:  docker build -t vidya-relay cloud/relay
# Run:    docker run -p 8788:8788 -e RELAY_SHARED_KEY=... vidya-relay
#
# TLS terminates at a reverse proxy in front of this container (see phase-5.md).

FROM rust:1-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin vidya-relay

FROM gcr.io/distroless/cc-debian12
COPY --from=build /build/target/release/vidya-relay /usr/local/bin/vidya-relay
ENV RELAY_BIND=0.0.0.0:8788
EXPOSE 8788
USER nonroot
ENTRYPOINT ["/usr/local/bin/vidya-relay"]
