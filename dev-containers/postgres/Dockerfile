FROM tensorchord/pgvecto-rs:pg16-v0.3.0

COPY ./create-postgres-databases.sh /docker-entrypoint-initdb.d/
RUN chmod +x /docker-entrypoint-initdb.d/create-postgres-databases.sh

