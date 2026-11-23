#!/usr/bin/env bash
# Builds the service image and proves a container made from it works: it serves
# an encrypted, authenticated compilation, refuses an unidentified caller,
# exposes metrics, runs as a non-root user and stops cleanly.
#
# Needs Docker, openssl, curl and the Rust toolchain (for the example client).
# Run scripts/vendor-sync.sh first: the image is built from the pinned upstream
# sources in the build context. Nothing is published; the image tag exists only
# on this machine and is removed afterwards.

set -euo pipefail
cd "$(dirname "$0")/.."

IMAGE="pp-cube-engine-service:check-$$"
NAME="pp-cube-engine-image-check-$$"
WORK=$(mktemp -d)
UPSTREAM_MANIFEST="vendor/cube/rust/cube/Cargo.toml"
fail() { echo "check image: FAILED — $1" >&2; exit 1; }
cleanup() {
  docker rm --force "$NAME" >/dev/null 2>&1 || true
  docker image rm --force "$IMAGE" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

[[ -f "$UPSTREAM_MANIFEST" ]] || fail "the pinned upstream is not checked out; run scripts/vendor-sync.sh"

echo "image: building" >&2
docker build --quiet --tag "$IMAGE" . >/dev/null || fail "build"
cargo build --quiet --locked -p pp-cube-service --example compile || fail "example client"
CLIENT=target/debug/examples/compile

# The container runs as a non-root user, so what it reads must be world-readable.
chmod 755 "$WORK"
cat > "$WORK/openssl.cnf" <<'CONFIG'
[req]
distinguished_name=dn
x509_extensions=ext
prompt=no
[dn]
CN=localhost
[ext]
subjectAltName=DNS:localhost,IP:127.0.0.1
basicConstraints=critical,CA:FALSE
CONFIG
openssl req -x509 -newkey rsa:2048 -nodes -days 2 -config "$WORK/openssl.cnf" \
  -keyout "$WORK/key.pem" -out "$WORK/cert.pem" >/dev/null 2>&1 || fail "certificate"
openssl rand -hex 16 > "$WORK/read.token"
cat > "$WORK/model.yaml" <<'MODEL'
cubes:
- name: sales
  data_source: warehouse
  sql_table: dw.sales
  measures:
  - {name: amount, type: sum, sql: "{CUBE}.amount"}
MODEL
chmod 644 "$WORK"/*

echo "image: starting a container" >&2
docker run --detach --name "$NAME" \
  --publish 127.0.0.1::50051 --publish 127.0.0.1::9100 \
  --volume "$WORK:/config:ro" "$IMAGE" \
  --model v1=/config/model.yaml --data-source warehouse=presto \
  --listen 0.0.0.0:50051 --metrics-listen 0.0.0.0:9100 \
  --tls-cert /config/cert.pem --tls-key /config/key.pem \
  --token-file /config/read.token >/dev/null || fail "start"
for _ in $(seq 1 60); do
  docker logs "$NAME" 2>/dev/null | grep -q '^listening ' && break
  [[ "$(docker inspect --format '{{.State.Running}}' "$NAME")" == true ]] || fail "the container exited"
  sleep 0.5
done
docker logs "$NAME" 2>/dev/null | grep -q '^listening ' || fail "no readiness line"
port=$(docker port "$NAME" 50051/tcp | head -1 | sed 's/.*://')
metrics_port=$(docker port "$NAME" 9100/tcp | head -1 | sed 's/.*://')
query='{"measures":["sales.amount"],"limit":5,"offset":5}'

echo "image: compiling through the container" >&2
compiled=$("$CLIENT" "https://127.0.0.1:$port" v1 presto "$query" "$WORK/cert.pem" "$WORK/read.token") \
  || fail "an identified caller was not served"
grep -q '^SELECT ' <<<"$compiled" || fail "no SQL returned"
grep -q -- '-- session: offset_clause_enabled=true' <<<"$compiled" || fail "no session property returned"
if refused=$("$CLIENT" "https://127.0.0.1:$port" v1 presto "$query" "$WORK/cert.pem" 2>&1); then
  fail "a caller without a token was served"
fi
grep -q 'Unauthenticated' <<<"$refused" || fail "a caller without a token was not refused as unauthenticated"
metrics=$(curl --silent --fail --max-time 5 "http://127.0.0.1:$metrics_port/metrics") || fail "metrics"
grep -q 'ppcube_requests_total{rpc="Compile",outcome="ok"} 1' <<<"$metrics" || fail "the compilation was not counted"

[[ "$(docker inspect --format '{{.Config.User}}' "$NAME")" == nonroot ]] || fail "the container does not run as nonroot"
docker stop --time 20 "$NAME" >/dev/null || fail "stop"
[[ "$(docker inspect --format '{{.State.ExitCode}}' "$NAME")" == 0 ]] || fail "the container did not stop cleanly"

echo "check image: PASSED — built, served an encrypted authenticated compilation, refused an unidentified caller, exposed metrics, ran as nonroot, stopped cleanly"
