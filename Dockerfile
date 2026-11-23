# The pp-cube-engine service image.
#
#   scripts/vendor-sync.sh          # the build copies vendor/cube from the context
#   docker build -t pp-cube-engine-service .
#
# The image holds one statically configured binary and no shell. It runs as a
# non-root user and takes the options described in crates/pp-cube-service/README.md.

FROM rust:1.98.1-slim-bookworm AS build
WORKDIR /src
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates crates
COPY templates templates
COPY vendor vendor
# --locked: the image is built from exactly the dependency set that was checked.
RUN cargo build --release --locked -p pp-cube-service \
    && cp target/release/pp-cube-service /pp-cube-service

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /pp-cube-service /usr/local/bin/pp-cube-service
COPY LICENSE NOTICE /usr/share/doc/pp-cube-engine/
USER nonroot
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/pp-cube-service"]
