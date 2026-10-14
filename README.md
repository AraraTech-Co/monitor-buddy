# Monitor Buddy

Simple Docker monitoring across multiple machines.

- **Frontend** (React + Vite): dashboards and charts in the browser
- **Server** (Rust / Axum + SQLite): auth, host registry, agent hub; in production serves the built UI
- **Agent** (Rust): lightweight process on each machine, talks to the local Docker socket and connects **outbound** over WebSocket

```
[Browser] --HTTP--> [Server+UI] <--WS-- [Agent] --unix--> [Docker]
                         |
                      [SQLite]
```

## Prerequisites

- Rust 1.80+
- Node.js 20+
- Docker (on machines you want to monitor)

## Quick start (local)

### 1. Start the server

```bash
cd monitor-buddy
mkdir -p data
export DATABASE_URL=sqlite:data/monitor-buddy.db
export FRONTEND_ORIGIN=http://localhost:5173
export PUBLIC_WS_URL=ws://localhost:8080/agent/ws
export BIND_ADDR=0.0.0.0:8080
cargo run -p server
```

### 2. Start the frontend

```bash
cd frontend
npm install
npm run dev
```

Open http://localhost:5173

For a single-process check (API + built UI), run `npm run build` in `frontend/` and set `STATIC_DIR=frontend/dist` plus `FRONTEND_ORIGIN=http://localhost:8080`.

### 3. Bootstrap user

On first run, use **Register bootstrap user** (only allowed when no users exist). Passwords are hashed with Argon2id.

### 4. Register a host

In **Hosts**, create a host. Copy the one-time `AGENT_TOKEN` and the suggested `docker run` command.

### 5. Run the agent

```bash
export SERVER_URL=ws://<server-host>:8080/agent/ws
export AGENT_TOKEN=<token-from-ui>
cargo run -p agent
```

Or with Docker (after the image is on Docker Hub or built locally):

```bash
docker run -d --name mb-agent \
  --restart unless-stopped \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e SERVER_URL=ws://<server-host>:8080/agent/ws \
  -e AGENT_TOKEN=<token> \
  <dockerhub-user>/<agent-repo>:latest
```

The agent only needs outbound access to the server (no inbound ports on the monitored host).

## Server environment

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDR` | `0.0.0.0:8080` | HTTP listen address |
| `DATABASE_URL` | `sqlite:data/monitor-buddy.db` | SQLite path |
| `FRONTEND_ORIGIN` | `http://localhost:5173` | CORS allowlist (use the public app URL in production) |
| `PUBLIC_WS_URL` | `ws://localhost:8080/agent/ws` | Shown in `docker run` helper |
| `STATIC_DIR` | `frontend/dist` | Built SPA directory (`/app/static` in the app image) |
| `COOKIE_SECURE` | `false` | Set `true` behind HTTPS |
| `SESSION_TTL_HOURS` | `72` | Session lifetime |

## Agent environment

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_URL` | `ws://localhost:8080/agent/ws` | WebSocket URL of the server |
| `AGENT_TOKEN` | *(required)* | Token issued when creating a host |
| `HEARTBEAT_SECS` | `10` | Snapshot interval |

## Docker images

Two images:

| Image | Dockerfile | Contents |
|-------|------------|----------|
| App | `docker/Dockerfile.server` | Backend + built frontend |
| Agent | `docker/Dockerfile.agent` | Agent only |

```bash
docker build -f docker/Dockerfile.server -t monitor-buddy:latest .
docker build -f docker/Dockerfile.agent -t monitor-buddy-agent:latest .

docker run -d --name monitor-buddy --restart unless-stopped \
  -p 8080:8080 \
  -v monitor-buddy-data:/data \
  -e FRONTEND_ORIGIN=http://localhost:8080 \
  -e PUBLIC_WS_URL=ws://localhost:8080/agent/ws \
  monitor-buddy:latest
```

The named volume `monitor-buddy-data` keeps the SQLite database across container recreates.

## Deploy (GitHub Actions → Docker Hub → SSH)

Workflow: [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml)

Triggers on push to `main` and `workflow_dispatch`.

1. Builds and pushes **app** and **agent** images to Docker Hub (`latest` + `sha-<short>`)
2. SSHs into the central server, `docker pull`s the app image, and recreates the container

### Repository secrets

| Secret | Description |
|--------|-------------|
| `DOCKERHUB_PASSWORD` | Docker Hub password or access token |
| `SSH_PRIVATE_KEY` | Private key (PEM) for SSH deploy; public key must be in `authorized_keys` on the server |

### Repository variables

| Variable | Required | Description |
|----------|----------|-------------|
| `DOCKERHUB_USERNAME` | yes | Docker Hub user/org |
| `DOCKERHUB_REPOSITORY` | yes | App image repo (e.g. `monitor-buddy`) |
| `DOCKERHUB_AGENT_REPOSITORY` | yes | Agent image repo (e.g. `monitor-buddy-agent`) |
| `SSH_USERNAME` | yes | SSH user on the central server |
| `SSH_HOST` | yes | Central server hostname/IP |
| `SSH_PORT` | no | Default `22` |
| `APP_CONTAINER_NAME` | no | Default `monitor-buddy` |
| `APP_HOST_PORT` | no | Host port mapped to container `8080` (default `8080`) |
| `FRONTEND_ORIGIN` | no | Public app URL (default `http://$SSH_HOST:$APP_HOST_PORT`) |
| `PUBLIC_WS_URL` | no | Agent WebSocket URL shown in UI (default `ws://...`) |
| `COOKIE_SECURE` | no | Default `false`; use `true` with HTTPS |

Example for `https://control.arara-tech.com` (proxy/TLS + host port `19642`):

| Variable | Value |
|----------|-------|
| `APP_HOST_PORT` | `19642` |
| `FRONTEND_ORIGIN` | `https://control.arara-tech.com` |
| `PUBLIC_WS_URL` | `wss://control.arara-tech.com/agent/ws` |
| `COOKIE_SECURE` | `true` |

The reverse proxy must forward HTTP and **WebSocket** upgrades to `127.0.0.1:19642`. Agents use `SERVER_URL=wss://control.arara-tech.com/agent/ws`.

Agents on other machines are **not** auto-deployed; pull the agent image when you update them:

```bash
docker pull <dockerhub-user>/<agent-repo>:latest
docker stop mb-agent && docker rm mb-agent
# re-run with the same AGENT_TOKEN / SERVER_URL as before
```

## API (MVP)

| Method | Path | Auth |
|--------|------|------|
| POST | `/api/auth/register` | none (first user only) |
| POST | `/api/auth/login` | none |
| POST | `/api/auth/logout` | session |
| GET | `/api/auth/me` | session |
| GET/POST | `/api/hosts` | session |
| DELETE | `/api/hosts/:id` | session |
| GET | `/api/hosts/:id/containers` | session |
| GET | `/api/hosts/:id/containers/:cid/logs?tail=200` | session |
| POST | `/api/hosts/:id/containers/:cid/restart` | session |
| GET | `/agent/ws` | agent token over WS |

## Project layout

```
crates/shared   shared DTOs / WS protocol
crates/server   central API + agent hub + optional static UI
crates/agent    per-machine Docker agent
frontend/       React UI
docker/         Dockerfiles
.github/        Deploy workflow
```

## Notes

- Container state is cached in memory from agent heartbeats; SQLite stores users, sessions, and hosts.
- Put TLS termination (Caddy/nginx) in front for production; set `COOKIE_SECURE=true` and `wss://` for agents.
