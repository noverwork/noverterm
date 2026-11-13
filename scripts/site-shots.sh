#!/usr/bin/env bash
# Re-shoots the landing page screenshots in docs/assets/shots.
# Needs macOS (headless Metal rendering), Docker for the demo SSH hosts, and cwebp.
set -euo pipefail
cd "$(dirname "$0")/.."

IMAGE=lscr.io/linuxserver/openssh-server:latest
HOSTS="web-01:2201 web-02:2202 api:2203"
RAW=target/site-shots
SHOTS=docs/assets/shots

cleanup() {
  for entry in $HOSTS; do docker rm -f "noverterm-site-${entry%%:*}" >/dev/null 2>&1 || true; done
}
trap cleanup EXIT
cleanup

keys=()
for entry in $HOSTS; do
  name=${entry%%:*} port=${entry##*:}
  docker run -d --name "noverterm-site-$name" --hostname "$name" -p "127.0.0.1:$port:2222" \
    -e PUID=1000 -e PGID=1000 -e USER_NAME=deploy -e PASSWORD_ACCESS=true -e USER_PASSWORD=demo \
    "$IMAGE" >/dev/null
done
for entry in $HOSTS; do
  name=${entry%%:*} port=${entry##*:}
  until fingerprint=$(ssh-keyscan -t ed25519 -p "$port" 127.0.0.1 2>/dev/null | ssh-keygen -lf - 2>/dev/null | awk '{print $2}') && [ -n "$fingerprint" ]; do
    sleep 1
  done
  keys+=("$port=$fingerprint")
  # Something to look at in the SFTP page's remote pane.
  docker exec "noverterm-site-$name" sh -c '
    cd /config && mkdir -p app/releases backups logs
    head -c 52428800 /dev/zero > app/releases/release-2026.11.03.tar.gz
    printf "server {}\n" > app/nginx.conf
    chown -R deploy:deploy app backups' >/dev/null
done
docker exec noverterm-site-web-01 apk add --no-cache htop >/dev/null

rm -rf "$RAW"
SITE_HOST_KEYS=$(IFS=,; echo "${keys[*]}") NOVERTERM_SITE_SHOTS=1 \
  cargo test -p desktop --test screenshots

# name: crop x y width height (in pixels of the 2x render), then output width
while read -r name x y w h out; do
  [ -z "$name" ] && continue
  src=$RAW/$name.png; [ "$name" = tabs ] && src=$RAW/hero.png
  cwebp -quiet -q 88 -crop "$x" "$y" "$w" "$h" -resize "$out" 0 "$src" -o "$SHOTS/$name.webp"
  echo "$SHOTS/$name.webp"
done <<'CROPS'
hero        0    0    2880 1620 1600
connections 0    0    2880 1620 1200
sftp        0    0    2880 1620 1440
mcp         0    0    2880 1520 1600
links       0    0    2880 1110 1440
forwards    200  310  880  460  716
snippets    210  320  2570 670  1440
keys        210  150  2570 400  1440
held        975  587  1047 474  548
tabs        120  4    2440 62   1232
CROPS
