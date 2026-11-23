# The pp-cube-engine service image.
#
#   scripts/vendor-sync.sh          # the build copies vendor/cube from the context
#   docker build -t pp-cube-engine-service .
#
# The image holds one statically configured binary and no shell. It runs as a
# non-root user and takes the options described in crates/pp-cube-service/README.md.

# Base images are pinned by digest; the tag is kept for readers. Update both together.
FROM rust:1.98.1-slim-bookworm@sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730 AS build
WORKDIR /src
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates crates
COPY templates templates
COPY vendor vendor
# --locked: the image is built from exactly the dependency set that was checked.
RUN cargo build --release --locked -p pp-cube-service \
    && cp target/release/pp-cube-service /pp-cube-service

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=build /pp-cube-service /usr/local/bin/pp-cube-service
COPY LICENSE NOTICE /usr/share/doc/pp-cube-engine/
USER nonroot
EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/pp-cube-service"]
