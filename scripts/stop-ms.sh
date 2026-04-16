#!/usr/bin/env sh

color="\033[0;35m"
reset="\033[1;37m"
working_dir="$(pwd)/dev-containers"
cd "$working_dir" || exit

echo "$color>>> Stop and remove containers$reset"

if [ "$1" = "--delete" ]; then
  docker compose down -v --remove-orphans
  echo "$color>>> Containers and volumes removed$reset"
else
  docker compose down
  echo "$color>>> Containers stopped$reset"
fi
