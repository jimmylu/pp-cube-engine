# Local gRPC service

Run one validated model version on an explicit local socket:

```sh
cargo run -p pp-cube-service -- \
  --model model.yaml --version v1 --digest local-model \
  --listen 127.0.0.1:50051 --workers 2 --queue 16
```

The model file accepts UTF-8 YAML or JSON. Version and digest are required nonblank
identifiers; digest is opaque caller metadata, not a verified content hash. The
listen address must be a literal loopback address. Port zero chooses a free port.
Worker and queue capacities must be positive; their defaults are 2 and 16.

After validation and binding, stdout prints `listening <actual-address>`.
Startup failure writes a diagnostic to stderr and exits unsuccessfully. Ctrl-C
stops the server gracefully. Clients use `ppcube.engine.v1.CubeEngine` from
`proto/engine.proto`: Compile, ValidateModel and GetMeta. Compile names exactly
the startup version and supplies dialect, timezone and trusted security filters.
The service compiles SQL without executing it.

This process is for trusted local callers, who can choose security context. It
has no authentication or TLS. To integrate remote access, use the library's
`GrpcService` in a host that owns those controls. No hot reload or additional
model versions are loaded by this executable.
