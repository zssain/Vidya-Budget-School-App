# Vidya relay (`cloud/relay`)

A small, **stateless** reverse tunnel that lets staff devices reach a school server
from anywhere (mobile data, home Wi‑Fi) without the school opening router ports
(docs §8.3 route 2, prompts/P05 Step 1). It is a **separate binary** and is **never
shipped inside the apps**.

## What it does

- The school server keeps ONE outbound WebSocket to `GET /tunnel/<school_id>`,
  authenticated with its **relay secret** (`Authorization: Bearer <secret>`), where
  `relay_secret = base64(HMAC-SHA256(RELAY_SHARED_KEY, school_id))` — the same value
  `cloud/licence` issues at activation. The relay recomputes it locally, so it never
  calls the licence service and keeps **no database**.
- Devices call `POST /s/<school_id>/v1/sealed`. The relay forwards the request as a
  frame over the tunnel and streams the response back.
- Every device body is **end‑to‑end sealed** between the device and the school
  (ChaCha20‑Poly1305, see the app's `sync::seal`). The relay sees only the school id,
  byte sizes, timings and status codes — it can read or alter **nothing**.

## Endpoints

| Route | Who | Notes |
|---|---|---|
| `GET /healthz` | infra | liveness → `200 ok` |
| `GET /tunnel/:school_id?epoch=<n>` | school server | WebSocket; `Bearer` relay secret; one tunnel per school, a newer epoch replaces the older (older gets close `4409 EPOCH_OLD`) |
| `ANY /s/:school_id/*path` | devices | forwarded over the tunnel; `503 SCHOOL_OFFLINE` when no tunnel |

Limits: 5 MB body, 64 concurrent requests/school (`429 SCHOOL_BUSY`), 240 req/10 s
per IP (`429 RATE_LIMITED`), 30 s forward timeout (`504 TIMEOUT`), 25 s keep‑alive
ping.

## Config (env)

| Var | Default | Meaning |
|---|---|---|
| `RELAY_BIND` | `0.0.0.0:8788` | bind address |
| `RELAY_SHARED_KEY` | dev default | shared key for relay‑secret verification — **must match `cloud/licence`** |

## TLS

Two supported options (Step 1 — "document both"):

1. **Reverse proxy (recommended, and what the Dockerfile assumes).** Terminate TLS
   at Caddy / nginx / a managed load balancer (Cloud Run, Fly, …) and proxy to the
   relay's plain `RELAY_BIND`. WebSocket upgrades pass through.
2. **In‑process rustls.** A follow‑up: add `RELAY_TLS_CERT`/`RELAY_TLS_KEY` PEM paths
   and serve with `tokio-rustls` (ring) using the same low‑level pattern as the
   school server's `net.rs`. Not enabled here to avoid an extra dependency while
   hosting is undecided (STOP condition — do not deploy this phase).

## Run

```sh
# local
RELAY_SHARED_KEY=dev cargo run --release

# docker
docker build -t vidya-relay cloud/relay
docker run -p 8788:8788 -e RELAY_SHARED_KEY=... vidya-relay
```

## Tests

`cargo test` covers the relay‑secret recipe, the body‑free access log (DONE #2),
forward round‑trip, `503` when offline, bad‑secret rejection, and newer‑epoch tunnel
replacement.
