# Cloud environment: one VPS, k3s

Brings the whole stack up on a single Linux server you already have (tested on Beget, Ubuntu 26.04, 4 vCPU / 8 GB / 40 GB). Terraform installs k3s and Calico over SSH, then reuses `modules/platform`, `modules/datastores` and `modules/rift` exactly as the local environment does; only `cluster.tf` is server-specific.

Requirements: SSH key access as root (password login can stay off); on the provider's firewall open 80/443, the LiveKit ports (`livekit_tcp_port`, 21 UDP ports from `livekit_udp_port`) and **TCP 6443** — Terraform's Kubernetes, kubectl and Helm providers talk to the API server directly, so with a deny-by-default firewall the targeted `null_resource.k3s` apply succeeds over SSH and the full apply then times out; restrict 6443 to the address that runs Terraform. Hostnames must resolve to the server. Without a domain use `sslip.io` names such as `rift.<ip>.sslip.io`; Let's Encrypt issues certificates for them.

```sh
cp terraform.tfvars.example terraform.tfvars   # fill in
terraform init
terraform apply -target=null_resource.k3s      # the providers need the cluster first
terraform apply
```

Editing `scripts/install-k3s.sh` re-runs it on the server, so fixes there reach existing clusters. The targeted apply is needed whenever `null_resource.k3s` itself changes, not only on the first run: while it is pending, the kubeconfig data source is unknown and the Kubernetes providers start with no credentials.

The kubeconfig lands in `~/.kube/rift-cloud.yaml` (`kubeconfig_path`; the directory is created if missing). `k3s_version` upgrades in place on the next apply, and a new `server_ip` for the same machine is reconciled in place too: the installer rewrites the server flags and drops the API server certificate so it is reissued for the new address. Changing `pod_subnet`, `service_subnet` or `cluster_domain` needs a rebuilt cluster, and the installer refuses rather than leaving k3s and Calico inconsistent. With `delivery = "argocd"` (plus `argocd_host` and a read-only `git_token`) Terraform installs Argo CD and hands the Rift release to it: the chart and `deploy/argocd/envs/staging/values.yaml` are synced from git, and every `vX.Y.Z` tag moves the stand to that version through the `promote` job. Images come from GHCR at `image_tag`; releases are cut by tagging `vX.Y.Z` in the repository.

What differs from a managed cluster: storage is k3s `local-path` on the server's disk (no backups, no HA), and the datastores run in the cluster. Swapping `modules/datastores` for managed PostgreSQL/Redis/object storage keeps everything else unchanged.
