# Rift Helm chart

Deploys the Rift backend to Kubernetes: four HTTP APIs (`identity-service`, `media-service`, `messaging-service`, `voice-service`), `realtime` (websocket + event dispatcher), the `outbox-worker`, `notification-service` and `livekit-pooler` workers, the `nsfw-detector` moderation service and an nginx gateway that carries the same routing as `docker/nginx/nginx.conf`.

Postgres, Redis, NATS JetStream, LiveKit and MinIO are external dependencies: point `config.*` at them. Set `livekit.externalName` to the LiveKit host if it lives outside the namespace so the gateway's `livekit:7880` upstream resolves.

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

`files/nginx.conf.template` is generated from `docker/nginx/nginx.conf` by `deploy/helm/sync-nginx.sh`; rerun it after changing the nginx config. The only substitution is the DNS resolver, which the container derives from `/etc/resolv.conf` at start.

## Usage

```sh
helm upgrade --install rift deploy/helm/rift \
  --namespace rift --create-namespace \
  --set image.tag=v1.2.3 \
  --set secrets.existingSecret=rift-secrets \
  --set config.POSTGRES_HOST=postgres.db.svc \
  --set gateway.ingress.enabled=true --set gateway.ingress.host=rift.example.com
```
