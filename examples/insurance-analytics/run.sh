#!/usr/bin/env bash
# Runs the example end to end: the compile service and a warehouse in containers,
# and the portal's back end asking its questions through both.
#
#   ./run.sh           start, load, ask, check, stop: the Java back end
#   ./run.sh python    the same with the Python back end
#   KEEP=1 ./run.sh    leave the containers running, and .run/, afterwards
#
# Needs Docker with Compose and openssl; a JDK 11 or later and Maven for the Java
# back end; Python 3.9 or later for the Python one. The service
# image is built from this checkout (run scripts/vendor-sync.sh once first), or
# set PPCUBE_IMAGE to a released image. Nothing is published, and everything
# it makes is under .run/ here.

set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd ../.. && pwd)"
RUN="$PWD/.run"
SECRETS="$RUN/secrets"

APP="${1:-java}"
case "$APP" in
  java) tools="docker openssl mvn java" ;;
  python) tools="docker openssl python3" ;;
  *) echo "usage: run.sh [java|python]" >&2; exit 64 ;;
esac
for tool in $tools; do
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

if [[ -z "${PPCUBE_IMAGE:-}" ]]; then
  [[ -f "$ROOT/vendor/cube/rust/cube/Cargo.toml" ]] \
    || { echo "example: run scripts/vendor-sync.sh first, or set PPCUBE_IMAGE" >&2; exit 2; }
  echo "example: building the service image (minutes the first time)" >&2
  docker compose build --quiet pp-cube-engine
fi
echo "example: starting the service and the warehouse" >&2
docker compose up --detach --quiet-pull --no-build >/dev/null

port() { docker compose port "$1" "$2" | sed 's/.*://'; }

export PPCUBE_TARGET="localhost:$(port pp-cube-engine 50051)"
export PPCUBE_CA="$SECRETS/tls.crt"
export PPCUBE_TOKEN_FILE="$SECRETS/read.token"
export WAREHOUSE_URL="jdbc:presto://localhost:$(port warehouse 8080)/memory/dw"
export WAREHOUSE_SQL="$PWD/warehouse/insurance.sql"
export PORTAL_FILE="$PWD/portal.json"

set +e
if [[ "$APP" == java ]]; then
  echo "example: building the client and the portal" >&2
  mvn --quiet --file "$ROOT/clients/java/pom.xml" install -DskipTests \
    && mvn --quiet --file app/pom.xml compile \
    && { echo "example: asking" >&2; mvn --quiet --file app/pom.xml exec:java; }
  status=$?
else
  # An environment of its own, with the client built from this checkout.
  echo "example: building the client and the portal" >&2
  # A current pip builds the client where it is, beside the contract it is generated from.
  python3 -m venv "$RUN/venv" \
    && "$RUN/venv/bin/pip" --quiet --disable-pip-version-check install --upgrade pip \
    && "$RUN/venv/bin/pip" --quiet --disable-pip-version-check install \
         "$ROOT/clients/python" --requirement app-python/requirements.txt \
    && { echo "example: asking" >&2; "$RUN/venv/bin/python" -W ignore::Warning app-python/portal.py; }
  status=$?
fi
set -e
if [[ $status -ne 0 ]]; then
  echo "example: the service's last lines:" >&2
  docker compose logs --tail 20 pp-cube-engine >&2 || true
fi
exit $status
