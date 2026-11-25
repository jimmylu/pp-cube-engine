#!/usr/bin/env bash
# Runs the example end to end: the compile service and a warehouse in containers,
# and the portal's back end asking its questions through both.
#
#   ./run.sh           start, load, ask, check, stop
#   KEEP=1 ./run.sh    leave the containers running, and .run/, afterwards
#
# Needs Docker with Compose, a JDK 11 or later, Maven and openssl. The service
# image is built from this checkout (run scripts/vendor-sync.sh once first), or
# set CINDERVALE_IMAGE to a released image. Nothing is published, and everything
# it makes is under .run/ here.

set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd ../.. && pwd)"
RUN="$PWD/.run"
SECRETS="$RUN/secrets"

for tool in docker mvn java openssl; do
  command -v "$tool" >/dev/null || { echo "example: $tool is needed" >&2; exit 2; }
done

stop() {
  if [[ "${KEEP:-}" != "1" ]]; then
    docker compose down --volumes --remove-orphans >/dev/null 2>&1 || true
    # The key and the tokens were for this run.
    rm -rf "$RUN"
  fi
}
trap stop EXIT

# What an operator provides as a secret: a certificate and two tokens. Made
# here for this run only; the container runs as another user, so it must be
# able to read them.
rm -rf "$RUN" && mkdir -p "$SECRETS"
cat > "$RUN/openssl.cnf" <<'CONFIG'
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
openssl req -x509 -newkey rsa:2048 -nodes -days 2 -config "$RUN/openssl.cnf" \
  -keyout "$SECRETS/tls.key" -out "$SECRETS/tls.crt" >/dev/null 2>&1
openssl rand -hex 24 > "$SECRETS/read.token"
openssl rand -hex 24 > "$SECRETS/admin.token"
chmod 755 "$RUN" "$SECRETS" && chmod 644 "$SECRETS"/*

if [[ -z "${CINDERVALE_IMAGE:-}" ]]; then
  [[ -f "$ROOT/vendor/cube/rust/cube/Cargo.toml" ]] \
    || { echo "example: run scripts/vendor-sync.sh first, or set CINDERVALE_IMAGE" >&2; exit 2; }
  echo "example: building the service image (minutes the first time)" >&2
  docker compose build --quiet pp-cube-engine
fi
echo "example: starting the service and the warehouse" >&2
docker compose up --detach --quiet-pull --no-build >/dev/null

port() { docker compose port "$1" "$2" | sed 's/.*://'; }

echo "example: building the client and the portal" >&2
mvn --quiet --file "$ROOT/clients/java/pom.xml" install -DskipTests
mvn --quiet --file app/pom.xml compile

export CINDERVALE_TARGET="localhost:$(port pp-cube-engine 50051)"
export CINDERVALE_CA="$SECRETS/tls.crt"
export CINDERVALE_TOKEN_FILE="$SECRETS/read.token"
export WAREHOUSE_URL="jdbc:presto://localhost:$(port warehouse 8080)/memory/dw"
export WAREHOUSE_SQL="$PWD/warehouse/insurance.sql"
export PORTAL_FILE="$PWD/portal.json"

echo "example: asking" >&2
set +e
mvn --quiet --file app/pom.xml exec:java
status=$?
set -e
if [[ $status -ne 0 ]]; then
  echo "example: the service's last lines:" >&2
  docker compose logs --tail 20 pp-cube-engine >&2 || true
fi
exit $status
