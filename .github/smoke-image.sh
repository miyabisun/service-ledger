#!/usr/bin/env bash
# Run the image on an empty volume: register a fictional service, restart, and read it back.
set -euo pipefail
image=${1:?usage: smoke-image.sh IMAGE}
container=$(docker run --detach --publish 127.0.0.1::3000 "$image")
trap 'docker rm --force --volumes "$container" >/dev/null' EXIT
base="http://$(docker port "$container" 3000/tcp)"

wait_healthy() {
  for attempt in {1..40}; do
    if curl --fail --silent "$base/healthz" >/dev/null; then
      return
    fi
    if [[ "$attempt" == 40 ]]; then
      docker logs "$container"
      exit 1
    fi
    sleep 0.5
  done
}

wait_healthy
test "$(curl --fail --silent "$base/api/health")" = '{"status":"ok"}'
curl --fail --silent --output /dev/null --header 'content-type: application/json' \
  --data '{"name":"Smoke Example Service","website_url":"https://smoke.example.com","source":"smoke test"}' \
  "$base/api/services"
docker exec "$container" sh -c 'test "$(id -u)" != 0 && test -f /app/data/service-ledger.db && test -f /app/data/service-ledger.db-wal && test ! -e /app/client'
docker restart "$container" >/dev/null
base="http://$(docker port "$container" 3000/tcp)"
wait_healthy
curl --fail --silent "$base/api/services?q=smoke" | grep -q '"name":"Smoke Example Service"'
test "$(curl --silent --output /dev/null --write-out '%{http_code}' "$base/services/1")" = "200"
echo "image smoke test passed"
