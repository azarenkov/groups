# Rift Helm chart

Deploys the Rift backend to Kubernetes: four HTTP APIs (`identity-service`, `media-service`, `messaging-service`, `voice-service`), `realtime` (websocket + event dispatcher), the `outbox-worker`, `notification-service` and `livekit-pooler` workers, the `nsfw-detector` moderation service and an nginx gateway that carries the same routing as `docker/nginx/nginx.conf`.

Postgres, Redis, NATS JetStream, LiveKit and MinIO are external dependencies: point `config.*` at them. Set `livekit.externalName` to the LiveKit host if it lives outside the namespace so the gateway's `livekit:7880` upstream resolves.

Voice clients connect to `config.LIVEKIT_CLIENT_URL`. The gateway proxies LiveKit on its `livekit` port (7880), and with `gateway.ingress.enabled` a second rule for `gateway.ingress.livekitHost` routes that host to it, so `LIVEKIT_CLIENT_URL` should be `wss://<livekitHost>` (or a separately published LiveKit endpoint). WebSocket upgrades on `/ws` and the LiveKit host need an ingress controller that supports them; for ingress-nginx set the read/send timeout annotations accordingly.

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

`secrets.*` are rendered into `<release>-secrets` unless `secrets.existingSecret` names a Secret you manage yourself; keys must match the environment variable names (`JWT_SECRET`, `POSTGRES_PASSWORD`, `LIVEKIT_API_SECRET`, `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD`, `MAILER_USERNAME`, `MAILER_PASSWORD`, `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `STEAM_API_KEY`).

## Gateway config

`files/nginx.conf.template` is generated from `docker/nginx/nginx.conf` by `deploy/helm/sync-nginx.sh`; rerun it after changing the nginx config. The script adapts the config for the unprivileged nginx image (listen on 8080, pid and temp paths under `/tmp`, logs to stdout/stderr) and leaves the DNS resolver as a placeholder that the container fills from `/etc/resolv.conf` at start. The gateway runs as uid 101 with a read-only root filesystem like every other workload. Upstreams are rewritten to `<service>.<namespace>.svc.<gateway.clusterDomain>` (nginx's own resolver ignores the pod search domains; set `gateway.resolverIpv6: "on"` on IPv6-only or dual-stack clusters so AAAA records are queried), and `gateway.trustedProxies` lists the CIDRs whose `X-Forwarded-For` is trusted to recover the client address before the rate and connection limits apply. It is empty by default, so limits and audit addresses use the direct peer; set it to exactly your ingress-controller pod or node CIDRs when the gateway sits behind an Ingress, never to whole private ranges, because any trusted source can pick its own client address. The gateway forwards only the sanitized `$remote_addr` upstream.

## Usage

```sh
helm upgrade --install rift deploy/helm/rift \
  --namespace rift --create-namespace \
  --set image.tag=v1.2.3 \
  --set secrets.existingSecret=rift-secrets \
  --set config.POSTGRES_HOST=postgres.db.svc \
  --set gateway.ingress.enabled=true --set gateway.ingress.host=rift.example.com
```
