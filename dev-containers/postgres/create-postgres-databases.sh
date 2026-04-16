#!/bin/bash

set -e
set -u

function create_databases() {
  local database=$1
  echo "Creating database '$database'"
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
	    CREATE DATABASE $database;
	    GRANT ALL PRIVILEGES ON DATABASE $database TO "$POSTGRES_USER";
EOSQL
}

if [ -n "${POSTGRES_DATABASES:-}" ]; then
  echo "Multiple database creation requested: $POSTGRES_DATABASES"
  for db in $(echo "$POSTGRES_DATABASES" | tr ',' ' '); do
    create_databases "$db"
  done
  echo "Multiple databases created"
fi
