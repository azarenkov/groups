# Cloud environment

This directory is the slot for a managed cluster. It reuses `modules/platform`, `modules/datastores` (or a managed replacement with the same outputs) and `modules/rift`; only the pieces that depend on a provider are missing:

1. `cluster.tf` — create the Kubernetes cluster and node pools with the provider of choice, then configure the `kubernetes` and `helm` providers from its kubeconfig. Label one node pool `rift.dev/edge=true` for LiveKit and the ingress controller, or run them everywhere with a `LoadBalancer` service.
2. `datastores` — either keep `modules/datastores` on cluster storage, or write a `modules/datastores-<provider>` with the same outputs (`postgres`, `redis_url`, `minio`, `namespace`) backed by managed PostgreSQL, Redis and object storage. `modules/rift` does not care which one it gets.
3. `dns.tf` — records for `app_host`, `livekit_host` and `argocd_host` pointing at the ingress address; cert-manager is switched to `issuer = "letsencrypt"` with `acme_email`.
4. `backend.tf` — remote state (object storage bucket plus a lock).
5. `trusted_proxies` for `modules/rift` — the ingress-controller pod CIDR (or node CIDR with host networking); with a cloud load balancer that sets `X-Forwarded-For`, also `ingress_nginx.forwarded_headers = true` and the balancer's addresses in `ingress_nginx.real_ip_cidrs`.

All three modules take `cluster_domain` (default `cluster.local`); set it on each when the cluster uses another DNS domain, because the gateway and LiveKit reach services through fully qualified names.

With `delivery = "argocd"` pass `argocd_ready = module.platform.argocd_release` to `modules/rift`, so the Application is created only after Argo CD's CRDs exist.

`main.tf` stays a copy of `envs/local/main.tf` with `delivery = "argocd"`, `git_token` set and the replicas from the chart defaults.
