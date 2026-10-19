variable "server_ip" {
  description = "Public IPv4 of the VPS that runs the single-node k3s cluster"
  type        = string
}

variable "node_ip" {
  description = "Address the server holds on its own interface, when NAT makes it differ from server_ip; host-networked LiveKit reaches the cluster from it, so the network policies allow it. Empty means it is the same as server_ip"
  type        = string
  default     = ""
}

variable "ssh_user" {
  type    = string
  default = "root"
}

variable "ssh_private_key" {
  description = "Path to the private key that opens the VPS"
  type        = string
}

variable "k3s_version" {
  type    = string
  default = "v1.32.7+k3s1"
}

variable "pod_subnet" {
  type    = string
  default = "10.42.0.0/16"
}

variable "service_subnet" {
  type    = string
  default = "10.43.0.0/16"
}

variable "kubeconfig_path" {
  description = "Where to write the cluster's kubeconfig; the directory is created if it does not exist"
  type        = string
  default     = "~/.kube/rift-cloud.yaml"
}

variable "cluster_domain" {
  type    = string
  default = "cluster.local"
}

variable "app_host" {
  description = "Host that serves the app; with no domain use <name>.<ip>.sslip.io"
  type        = string
}

variable "livekit_host" {
  type = string
}

variable "grafana_host" {
  type    = string
  default = ""
}

variable "acme_email" {
  description = "Let's Encrypt account email"
  type        = string
}

variable "image_registry" {
  type    = string
  default = "ghcr.io/rift-space/rift"
}

variable "image_tag" {
  type = string
}

variable "ghcr_user" {
  type    = string
  default = ""
}

variable "ghcr_token" {
  type      = string
  default   = ""
  sensitive = true
}

variable "livekit_tcp_port" {
  type    = number
  default = 7881
}

variable "livekit_udp_port" {
  description = "First of 21 UDP ports LiveKit binds on the host"
  type        = number
  default     = 50000
}

variable "delivery" {
  description = "helm: Terraform installs the chart; argocd: Argo CD syncs deploy/helm/rift from git with deploy/argocd/envs/staging/values.yaml, so tagged releases roll out on their own"
  type        = string
  default     = "helm"
}

variable "argocd_host" {
  type    = string
  default = ""
}

variable "git_revision" {
  type    = string
  default = "main"
}

variable "git_ssh_key" {
  description = "Private half of a read-only deploy key for the repository; Argo CD uses it instead of git_token. Pass it as TF_VAR_git_ssh_key so it stays out of the tfvars file"
  type        = string
  default     = ""
  sensitive   = true
}

variable "git_token" {
  description = "Token Argo CD uses to read the private repository (fine-grained, Contents: read on rift-space/rift)"
  type        = string
  default     = ""
  sensitive   = true
}

variable "monitoring" {
  type    = bool
  default = false
}

variable "mailer" {
  type = object({
    host     = optional(string, "smtp.example.com")
    port     = optional(string, "465")
    from     = optional(string, "noreply@example.com")
    username = optional(string, "")
    password = optional(string, "")
  })
  default   = {}
  sensitive = true
}
