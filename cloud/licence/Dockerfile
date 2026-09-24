# Vidya licence service — multi-stage build → distroless (glibc, non-root).
# Links rustls(ring)-free axum (no TLS in-process; Fly terminates TLS at the edge).
#
# Build:  docker build -t vidya-licence cloud/licence
# Run:    docker run -p 8787:8787 \
#           -e LICENCE_BIND=0.0.0.0 -e LICENCE_DATA_DIR=/data \
#           -e LICENCE_SIGNING_KEY=... -e RELAY_SHARED_KEY=... \
#           -v vidya-licence-data:/data vidya-licence
#
# Production env (set as Fly secrets / a mounted volume — see docs/DEPLOY-FLY.md):
#   LICENCE_SIGNING_KEY  base64 ed25519 seed (the PRIVATE key; `gen-prod-key`)
#   LICENCE_DATA_DIR     persistent store dir (a Fly volume, e.g. /data)
#   RELAY_SHARED_KEY     same value as the relay service
#   LICENCE_BIND/PORT    0.0.0.0 / 8787

FROM rust:1-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# migrations/ and assets/ are embedded at compile time (include_str!), so the
# build stage needs them present next to src/.
COPY migrations ./migrations
COPY assets ./assets
RUN cargo build --release --locked --bin vidya-licence

FROM gcr.io/distroless/cc-debian12
COPY --from=build /build/target/release/vidya-licence /usr/local/bin/vidya-licence
ENV LICENCE_BIND=0.0.0.0 \
    LICENCE_PORT=8787 \
    LICENCE_DATA_DIR=/data
EXPOSE 8787
USER nonroot
ENTRYPOINT ["/usr/local/bin/vidya-licence"]
CMD ["serve"]
