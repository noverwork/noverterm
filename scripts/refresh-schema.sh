#!/usr/bin/env sh

color="\033[0;35m"
reset="\033[1;37m"

export NODE_ENV="development"

echo "$color>>> Build backend$reset"
npm run backend:build

echo "$color>>> Refresh database schema$reset"
node dist/apps/backend/refresh-schema.js

if [ "$1" = "--no-seed" ]; then
  exit
fi

echo "$color>>> Seed data$reset"
node dist/apps/backend/seed.js
