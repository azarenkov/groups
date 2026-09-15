# Rift Helm chart

Deploys the Rift backend to Kubernetes: four HTTP APIs (`identity-service`, `media-service`, `messaging-service`, `voice-service`), `realtime` (websocket + event dispatcher), the `outbox-worker`, `notification-service` and `livekit-pooler` workers, the `nsfw-detector` moderation service and an nginx gateway that carries the same routing as `docker/nginx/nginx.conf`.

Postgres, Redis, NATS JetStream, LiveKit and MinIO are external dependencies: point `config.*` at them.

## Databases

Every service owns its own Postgres database and role; `databases.services.<service>` names them for `identity`, `media`, `messaging`, `voice` and `realtime`. `config.POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_OPTIONS` and the pool settings are shared, while each workload receives only the `<SERVICE>_POSTGRES_DB`, `<SERVICE>_POSTGRES_USER` and `<SERVICE>_POSTGRES_PASSWORD` variables of the databases listed in its `databases` value. The service creates its schema and runs its own migrations on start. `livekit-pooler` uses the `messaging` database, and `outbox-worker` relays the outboxes of `OUTBOX_SOURCES` with those services' credentials. Service passwords live in `<release>-databases` under `<service>-password` keys unless `databases.existingSecret` names a Secret you manage.

`databases.bootstrap` creates the databases and roles on a self-hosted Postgres. A `pre-install`/`pre-upgrade` hook Job connects as `databases.bootstrap.adminUser` (a role with `CREATEDB` and `CREATEROLE`), creates each missing role and database, makes the role the owner, sets the role password from the service password on every run so rotations reach the database, and revokes `CONNECT` from `PUBLIC` so one service cannot open another service's database. Nothing is dropped on uninstall. The admin password comes from `databases.bootstrap.adminPassword`, rendered into a hook Secret that is deleted after the Job succeeds, or from `databases.bootstrap.existingSecret` under `databases.bootstrap.existingSecretKey`. With bootstrap enabled the chart refuses to render without the admin password and non-empty service passwords. On a managed Postgres that does not allow `CREATE ROLE`, set `databases.bootstrap.enabled=false` and create the databases and roles with the provider's tools.

Upgrading a release that still uses the single shared `POSTGRES_DB` does not move any data: the services start on new, empty databases and the old database is left untouched.

## Public URLs

`config.PUBLIC_API_URL` is the origin of the gateway as clients reach it, and `config.PUBLIC_APP_URL` is the origin of the web app. Invite links are built as `<PUBLIC_APP_URL>/join/<token>` and `messaging-service` refuses to start without an `http(s)://` value, so replace the example domain with the host that serves the web app.

## Probes and shutdown

| Workload | Ready/startup | Live | Notes |
| --- | --- | --- | --- |
| HTTP APIs | `:8080/readyz` | `:8080/livez` | startup probe budgets `probes.startup.periodSeconds × failureThreshold` for migrations |
| realtime | `:8084/readyz` | `:8085/livez` | readiness from the websocket half, liveness from the dispatcher heartbeat |
| outbox-worker | `:8081` | `:8081` | heartbeat-backed liveness |
| notification-service | `:8082` | `:8082` | heartbeat-backed liveness |
| livekit-pooler | `:8086` | `:8086` | heartbeat-backed liveness |
| nsfw-detector | `:8000/readyz` | `:8000/livez` | readiness waits for model loading (`nsfwDetector.startupFailureThreshold`) |

Every pod gets `preStop: sleep shutdown.preStopSeconds`, then SIGTERM. The services flip `/readyz` to 503, wait `SHUTDOWN_DRAIN_MS`, stop accepting and give in-flight work `SHUTDOWN_TIMEOUT_SECS`; `shutdown.terminationGracePeriodSeconds` must exceed the sum.

## Images

Images come from `image.registry/<service>:image.tag` (defaults to `ghcr.io/rift-space/rift/<service>` and `Chart.appVersion`). All containers run as uid 10001 with a read-only root filesystem.

## Secrets

`secrets.*` are rendered into `<release>-secrets` unless `secrets.existingSecret` names a Secret you manage yourself; keys must match the environment variable names (`JWT_SECRET`, `LIVEKIT_API_SECRET`, `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD`, `MAILER_USERNAME`, `MAILER_PASSWORD`, `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `STEAM_API_KEY`).

## Gateway config

`files/nginx.conf.template` is generated from `docker/nginx/nginx.conf` by `deploy/helm/sync-nginx.sh`; rerun it after changing the nginx config. The script adapts the config for the unprivileged nginx image (listen on 8080, pid and temp paths under `/tmp`, logs to stdout/stderr) and leaves the DNS resolver as a placeholder that the container fills from `/etc/resolv.conf` at start. The gateway runs as uid 101 with a read-only root filesystem like every other workload. Upstreams are rewritten to `<service>.<namespace>.svc.<gateway.clusterDomain>` (nginx's own resolver ignores the pod search domains; set `gateway.resolverIpv6: "on"` on IPv6-only or dual-stack clusters so AAAA records are queried), and `gateway.trustedProxies` lists the CIDRs whose `X-Forwarded-For` is trusted to recover the client address before the rate and connection limits apply. It is empty by default, so limits and audit addresses use the direct peer; set it to exactly your ingress-controller pod or node CIDRs when the gateway sits behind an Ingress, never to whole private ranges, because any trusted source can pick its own client address. The gateway forwards only the sanitized `$remote_addr` upstream.

## Usage

```sh
helm upgrade --install rift deploy/helm/rift \
  --namespace rift --create-namespace \
  --set image.tag=v1.2.3 \
  --set secrets.existingSecret=rift-secrets \
  --set databases.existingSecret=rift-databases \
  --set databases.bootstrap.existingSecret=rift-postgres-admin \
  --set config.POSTGRES_HOST=postgres.db.svc \
  --set config.PUBLIC_APP_URL=https://app.rift.example.com \
  --set gateway.ingress.enabled=true --set gateway.ingress.host=rift.example.com
```
