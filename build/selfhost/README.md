# Self-hosting with Docker Compose

This directory runs the ArtCraft backend (`storyteller-web`) and the services it
depends on with Docker Compose.

| Service           | Image                    | Purpose                                     |
|-------------------|--------------------------|---------------------------------------------|
| `storyteller-web` | built from this repo     | HTTP API, on `http://localhost:12345`       |
| `mysql`           | `mysql:8.4`              | Accounts, media metadata, jobs, billing     |
| `migrate`         | `mysql:8.4`              | Applies `_database/sql/migrations`, exits   |
| `redis`           | `redis:7`                | Caching, rate limiting, job progress        |
| `elasticsearch`   | `elasticsearch:8.15.3`   | Search                                      |
| `seaweedfs`       | `chrislusf/seaweedfs`    | S3-compatible storage for media             |
| `storage-init`    | `chrislusf/seaweedfs`    | Creates the storage buckets, exits          |

Only the API port (`12345`) is published to the host.

## Requirements

- Docker with Docker Compose v2.23 or newer.
- About 8 GB of memory for Docker and 20 GB of free disk. The first build
  compiles the Rust workspace in release mode and can take a long time.

## Usage

From this directory:

```bash
cp .env.example .env    # then edit the passwords and secrets
docker compose up --build -d
```

Check that the server is up:

```bash
curl http://localhost:12345/_status
```

Stop everything with `docker compose down`. Add `-v` to also delete the
database, search index and stored media.

## Configuration

Settings live in `.env`, which git ignores. `docker-compose.yml` sets the
connection settings for MySQL, Redis, Elasticsearch and storage, so you only
need to change passwords and secrets.

`storyteller-web` requires credentials for every third-party provider (FAL,
OpenAI, Stripe, Resend and others) to be present at startup. `.env.example`
sets them to the placeholder `unset`, which lets the server start. Features
that call a provider only work after you add a real key for it.

Provider webhooks, such as `FAL_WEBHOOK_URL`, must be reachable from the
internet for asynchronous generation results to arrive.

## Database migrations

The `migrate` service applies every migration in `_database/sql/migrations` in
order and records each one in `__diesel_schema_migrations`, the table the diesel
CLI uses. It runs on every `docker compose up` and only applies migrations that
have not run yet.

## Connecting the web app

To point the browser app at this server, run it from `frontend` as described in
the repository README:

```bash
VITE_USE_LOCAL_API=true npx nx dev artcraft-webapp
```

## Limitations

- Only the HTTP API is included. Background workers (provider polling, video
  thumbnails, email, analytics) are not started.
- Elasticsearch starts with no indexes. Index definitions are in
  `_database/elasticsearch/index_definitions`.
