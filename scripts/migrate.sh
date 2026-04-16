#!/usr/bin/env sh

color="\033[0;35m"
alert="\033[0;31m"
reset="\033[1;37m"

export NODE_ENV="development"

echo "$color>>> Build backend$reset"
npm run backend:build

if [ "$1" = "create" ]; then
  echo "$color>>> Create migration $2$reset"
  node dist/apps/backend/migrate.js create "$2"
elif [ "$1" = "up" ]; then
  echo "$color>>> Migration up$reset"
  node dist/apps/backend/migrate.js up
elif [ "$1" = "down" ]; then
  echo "$color>>> Migration down$reset"
  node dist/apps/backend/migrate.js down
else
  echo "$alert>>> No command found"
fi
