# Monitor Buddy

Simple Docker monitoring across multiple machines.

- **Frontend** (React + Vite): dashboards and charts in the browser
- **Server** (Rust / Axum + SQLite): auth, host registry, agent hub
- **Agent** (Rust): lightweight process on each machine, talks to the local Docker socket and connects **outbound** over WebSocket

```
[Browser] --HTTP--> [Server] <--WS-- [Agent] --unix--> [Docker]
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

### 3. Bootstrap user

On first run, use **Register bootstrap user** (only allowed when no users exist). Passwords are hashed with Argon2id.

### 4. Register a host

In **Hosts**, create a host. Copy the one-time `AGENT_TOKEN` and the suggested `docker run` command.

### 5. Run the agent

Build the agent image (from repo root):

```bash
docker build -f docker/Dockerfile.agent -t monitor-buddy-agent:latest .
```

Or run the binary directly on a machine with Docker:

```bash
export SERVER_URL=ws://<server-host>:8080/agent/ws
export AGENT_TOKEN=<token-from-ui>
cargo run -p agent
```

With Docker:

```bash
docker run -d --name mb-agent \
  --restart unless-stopped \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e SERVER_URL=ws://<server-host>:8080/agent/ws \
  -e AGENT_TOKEN=<token> \
  monitor-buddy-agent:latest
```

The agent only needs outbound access to the server (no inbound ports on the monitored host).

## Server environment

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDR` | `0.0.0.0:8080` | HTTP listen address |
| `DATABASE_URL` | `sqlite:data/monitor-buddy.db` | SQLite path |
| `FRONTEND_ORIGIN` | `http://localhost:5173` | CORS + cookie origin |
| `PUBLIC_WS_URL` | `ws://localhost:8080/agent/ws` | Shown in `docker run` helper |
| `COOKIE_SECURE` | `false` | Set `true` behind HTTPS |
| `SESSION_TTL_HOURS` | `72` | Session lifetime |

## Agent environment

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_URL` | `ws://localhost:8080/agent/ws` | WebSocket URL of the server |
| `AGENT_TOKEN` | *(required)* | Token issued when creating a host |
| `HEARTBEAT_SECS` | `10` | Snapshot interval |

## Docker (server)

```bash
docker build -f docker/Dockerfile.server -t monitor-buddy-server:latest .
docker run -d --name mb-server -p 8080:8080 \
  -e FRONTEND_ORIGIN=http://localhost:5173 \
  -e PUBLIC_WS_URL=ws://localhost:8080/agent/ws \
  -v mb-data:/data \
  monitor-buddy-server:latest
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
crates/server   central API + agent hub
crates/agent    per-machine Docker agent
frontend/       React UI
docker/         Dockerfiles
```

## Notes

- Container state is cached in memory from agent heartbeats; SQLite stores users, sessions, and hosts.
- Put TLS termination (Caddy/nginx) in front for production; set `COOKIE_SECURE=true` and `wss://` for agents.
