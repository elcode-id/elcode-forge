# ELCODE Forge Server — Self-Hosting & Production Operations Runbook

**Product:** ELCODE Forge (`eforge`)  
**Component:** `crates/forge-server`  
**Target Release:** v1.0.0 (Phase 9B)  
**Organization:** [ELCODE](https://elcode.id/) (Independent Software Studio — Bandung, Indonesia)

---

## 1. Architecture Overview

ELCODE Forge Server is a lightweight, cloud-native synchronization and continuous code intelligence backend. It provides central repository workspace registration, multi-branch historical score retention, AST/dependency cache sharing, and outbound webhook dispatching.

```mermaid
flowchart TD
    CLI["eforge CLI / CI Agent (eforge sync, eforge push)"] -->|HTTPS / REST API| Ingress["Reverse Proxy / Ingress (Nginx / Caddy / K8s Ingress)"]
    Web["Web Intelligence Dashboard (:3000)"] -->|REST API| Ingress
    Ingress -->|Port :8080| Server["forge-server (Rust / Axum Engine)"]
    
    subgraph Data Layer
        Server -->|Connection Pool| DB["PostgreSQL 16 (or Embedded SQLite)"]
        Server -->|S3 Protocol| S3["MinIO / AWS S3 (Artifacts & Cache)"]
    end

    subgraph Outbound Integrations
        Server -->|HMAC-SHA256 Webhook| Slack["Slack Incoming Webhook"]
        Server -->|HMAC-SHA256 Webhook| Discord["Discord Webhook Embed"]
        Server -->|Generic Webhook| Generic["Custom Internal Services"]
    end
```

---

## 2. Hardware Requirements & Sizing

| Deployment Tier | Active Contributors | Recommended Database | CPU | Memory | Storage (SSD) | Typical Scan Throughput |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Personal / Small** | 1 – 15 | Embedded SQLite | 1 vCPU | 512 MB | 10 GB | ~200 snapshots/day |
| **Team / Medium** | 15 – 100 | PostgreSQL 16 | 2 vCPU | 2 GB | 50 GB | ~2,500 snapshots/day |
| **Enterprise / Large** | 100 – 500+ | HA PostgreSQL + MinIO | 4 vCPU | 8 GB | 200 GB+ | ~20,000 snapshots/day |

---

## 3. Quickstart Deployments

### Option A — Standalone Docker (Single-Container SQLite Mode)

For quick evaluations or small teams without a dedicated database server:

```bash
# 1. Create a persistent volume directory
mkdir -p /opt/eforge/data

# 2. Run forge-server container
docker run -d \
  --name forge-server \
  --restart unless-stopped \
  -p 8080:8080 \
  -v /opt/eforge/data:/var/lib/eforge \
  -e EFORGE_SERVER_HOST="0.0.0.0" \
  -e EFORGE_SERVER_PORT="8080" \
  -e EFORGE_DATABASE_URL="sqlite:///var/lib/eforge/forge.db" \
  -e EFORGE_JWT_SECRET="generate_at_least_32_characters_random_secret" \
  -e EFORGE_ADMIN_API_KEY="admin_bootstrap_key_12345" \
  elcode/forge-server:latest

# 3. Verify health status
curl -i http://localhost:8080/api/v1/health
```

---

### Option B — Production Docker Compose (Complete Multi-Service Stack)

The Docker Compose stack orchestrates `forge-server`, `postgres`, `minio`, and the `dashboard`:

```bash
# 1. Clone repository or download deployment files
git clone https://github.com/elcode-id/elcode-forge.git
cd elcode-forge

# 2. Copy and customize environment variables
cp .env.example .env
nano .env # Set secure passwords, EFORGE_JWT_SECRET, and EFORGE_ADMIN_API_KEY

# 3. Start stack in detached mode
docker compose up -d

# 4. Inspect service health
docker compose ps
```

The services will be accessible at:
- **Web Dashboard:** `http://localhost:3000`
- **Forge Server API:** `http://localhost:8080`
- **MinIO S3 Console:** `http://localhost:9001`
- **PostgreSQL Database:** `localhost:5432`

---

### Option C — Kubernetes Deployment (Cloud-Native Cluster)

Kubernetes manifests are preconfigured in `deploy/k8s/`:

```bash
# 1. Inspect and adjust secrets and domain names
nano deploy/k8s/secret.yaml
nano deploy/k8s/ingress.yaml

# 2. Deploy using Kustomize
kubectl apply -k deploy/k8s/

# 3. Monitor rollout status
kubectl rollout status deployment/forge-server -n forge
kubectl get pods,svc,ingress -n forge
```

---

## 4. Configuration Reference

All settings can be configured via environment variables:

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `EFORGE_SERVER_HOST` | `127.0.0.1` | Network interface IP address to bind (`0.0.0.0` for containers). |
| `EFORGE_SERVER_PORT` | `8080` | TCP port for HTTP API service. |
| `EFORGE_DATABASE_URL` | `sqlite://forge.db` | Database connection string (`sqlite://...` or `postgres://user:pass@host:5432/db`). |
| `EFORGE_JWT_SECRET` | *(Randomly generated)* | 32+ character HMAC secret for signing client access tokens. |
| `EFORGE_ADMIN_API_KEY` | *(Disabled if omitted)* | Static bootstrap key for provisioning API keys and webhooks. |
| `RUST_LOG` | `info` | Logging verbosity filter (e.g. `info,forge_server=debug`). |

---

## 5. Database Management, Backup & Restore

### PostgreSQL Automated Backup Runbook

```bash
# 1. Take a consistent logical snapshot
docker exec -t forge-postgres pg_dump -U forge -d forge_db -F c -b -v -f /tmp/forge_backup.dump
docker cp forge-postgres:/tmp/forge_backup.dump ./forge_backup_$(date +%Y%m%d_%H%M%S).dump

# 2. Restore from snapshot
docker cp ./forge_backup_20261023.dump forge-postgres:/tmp/restore.dump
docker exec -t forge-postgres pg_restore -U forge -d forge_db -c -v /tmp/restore.dump
```

### SQLite Backup Runbook

```bash
# Perform an atomic online backup using SQLite CLI
sqlite3 /opt/eforge/data/forge.db ".backup '/opt/eforge/backups/forge_$(date +%Y%m%d_%H%M%S).db'"
```

---

## 6. Production SSL/TLS Reverse Proxy Configurations

### Nginx Configuration

Save to `/etc/nginx/sites-available/forge.example.com`:

```nginx
server {
    listen 80;
    server_name forge.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name forge.example.com;

    ssl_certificate /etc/letsencrypt/live/forge.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/forge.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    client_max_body_size 64M;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Caddy Configuration

Add to `/etc/caddy/Caddyfile`:

```caddy
forge.example.com {
    reverse_proxy 127.0.0.1:8080 {
        header_up Host {host}
        header_up X-Real-IP {remote}
        header_up X-Forwarded-Proto {scheme}
    }
}
```

---

## 7. Air-Gapped & Offline Deployment

For secure environments without direct internet connectivity:

```bash
# 1. On an internet-connected workstation, pull and export images
docker pull elcode/forge-server:v1.0.0
docker pull postgres:16-alpine
docker pull minio/minio:RELEASE.2024-10-13T13-34-11Z

docker save \
  elcode/forge-server:v1.0.0 \
  postgres:16-alpine \
  minio/minio:RELEASE.2024-10-13T13-34-11Z | gzip > forge-server-airgap-bundle.tar.gz

# 2. Transfer bundle via approved secure storage media to target air-gapped machine

# 3. Load images into target Docker daemon
gunzip -c forge-server-airgap-bundle.tar.gz | docker load

# 4. Start stack using offline docker-compose
docker compose up -d
```

---

## 8. Security Best Practices

1. **Non-Root Execution:** The server container strictly enforces non-root execution as `UID 10001` (`forge:forge`) with `readOnlyRootFilesystem: true` in Kubernetes.
2. **Secret Rotation:** When rotating `EFORGE_JWT_SECRET`, existing client tokens will become invalid. Re-authenticate CLI agents via `eforge login`.
3. **Webhook HMAC Signatures:** Always configure a shared webhook secret (`secret`) when registering webhook endpoints. Validate the `X-Forge-Signature: sha256=<hex>` header on your receiver.
4. **Firewall Isolation:** Expose only port `443` (Reverse Proxy) externally. Keep PostgreSQL (`5432`) and MinIO (`9000`) bound strictly to `127.0.0.1` or internal overlay networks.

---

## 9. Observability & Troubleshooting

- **Health Probe:** `curl -sS http://localhost:8080/api/v1/health`  
  Returns `{ "status": "ok", "version": "1.0.0", "database": "sqlite"|"postgres" }`.
- **Database Migrations:** Schema migrations run automatically upon startup. If a migration fails due to lock timeout, check `RUST_LOG=debug` logs.
- **Log Inspection:**
  ```bash
  docker compose logs -f forge-server
  ```
