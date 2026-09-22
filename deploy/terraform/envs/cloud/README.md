# Cloud environment: one VPS, k3s

Brings the whole stack up on a single Linux server you already have (tested on Beget, Ubuntu 26.04, 4 vCPU / 8 GB / 40 GB). Terraform installs k3s and Calico over SSH, then reuses `modules/platform`, `modules/datastores` and `modules/rift` exactly as the local environment does; only `cluster.tf` is server-specific.

Requirements: SSH key access as root (password login can stay off), ports 80/443 and the LiveKit ports (`livekit_tcp_port`, 21 UDP ports from `livekit_udp_port`) open on the provider's firewall, and hostnames that resolve to the server. Without a domain use `sslip.io` names such as `rift.<ip>.sslip.io`; Let's Encrypt issues certificates for them.

```sh
cp terraform.tfvars.example terraform.tfvars   # fill in
terraform init
terraform apply -target=null_resource.k3s      # the providers need the cluster first
terraform apply
```

The kubeconfig lands in `~/.kube/rift-cloud.yaml`. Images come from GHCR at `image_tag`; releases are cut by tagging `vX.Y.Z` in the repository.

What differs from a managed cluster: storage is k3s `local-path` on the server's disk (no backups, no HA), and the datastores run in the cluster. Swapping `modules/datastores` for managed PostgreSQL/Redis/object storage keeps everything else unchanged.
