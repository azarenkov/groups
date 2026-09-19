# Deploying Rift

```
deploy/
  helm/rift/        application chart: services, workers, gateway, web, policies
  terraform/
    modules/platform    ingress-nginx, cert-manager, NATS, LiveKit, Argo CD, monitoring
    modules/datastores  Postgres, Redis, MinIO on cluster storage (swap for managed services)
    modules/rift        namespace, secrets and the Rift release (Helm or Argo CD)
    envs/local          kind cluster with Calico; everything above on a laptop
    envs/cloud          the slot for a managed cluster
  argocd/envs/        per-environment values that CI bumps on release
  scripts/smoke.sh    end-to-end check through the ingress
  Makefile
```

## Local cluster

Requirements: Docker, `kind`, `kubectl`, `helm`, `terraform`. Ports 80 and 443 on the host must be free; LiveKit needs one TCP and 21 UDP ports (defaults 7881 and 7882–7902; set `livekit_tcp_port`/`livekit_udp_port` in `envs/local/terraform.tfvars` when docker compose already uses them).

```sh
make -C deploy local-up
```

That builds every image as `rift/<service>:local`, creates the kind cluster, installs Calico, ingress-nginx, cert-manager, NATS, LiveKit, Postgres, Redis and MinIO, deploys the chart and runs the smoke test. The app is at `https://rift.localtest.me` (self-signed certificate). `make -C deploy local-down` destroys the cluster.

Reusing images already built by docker compose: `docker tag rift-identity-service:latest rift/identity-service:local` for each service, then `make -C deploy kind-load` and `terraform apply` in `envs/local`.

To pull released images from GHCR instead: `image_registry = "ghcr.io/rift-space/rift"`, `image_tag = "1.2.3"`, `ghcr_user`/`ghcr_token` with `read:packages`.

## Delivery modes

`modules/rift` either installs the chart itself (`delivery = "helm"`, the default for local) or creates an Argo CD `Application` (`delivery = "argocd"`) that syncs `deploy/helm/rift` from git with `deploy/argocd/envs/<env>/values.yaml` on top. Terraform still owns the namespace and the secrets in both modes; the chart only references them through `existingSecret`. See `argocd/README.md` for how CI moves the image tag.

## CI/CD

- `Infra` (`.github/workflows/infra.yml`): on pull requests touching `deploy/` — `helm lint`, `helm template`, `terraform fmt/validate`, web image build. Weekly and on demand (`cluster: true`) it brings the local environment up on a runner with the `latest` GHCR images and runs `scripts/smoke.sh`.
- `Release Services`: builds `ghcr.io/rift-space/rift/<service>` for every service and `web`; on a `vX.Y.Z` tag it then commits `image.tag: X.Y.Z` into `argocd/envs/staging/values.yaml` so Argo CD rolls the release out.
- `Release Web`: on the same tag builds the static web bundle and attaches `rift-web-X.Y.Z.tar.gz` to the release in `rift-space/rift-releases`, next to the desktop binaries electron-builder publishes there.
- `Release Desktop`: unchanged, publishes macOS/Windows/Linux builds to `rift-space/rift-releases`.

## When a cloud provider appears

Only `envs/cloud` changes: a `cluster.tf` for the managed Kubernetes, a datastores module with the same outputs backed by managed PostgreSQL/Redis/object storage, DNS records and a remote state backend. `modules/platform` and `modules/rift` are reused as they are; switch `cert_manager.issuer` to `letsencrypt` and `delivery` to `argocd`.
