variable "namespace" {
  type    = string
  default = "rift"
}

variable "release_name" {
  type    = string
  default = "rift"
}

variable "delivery" {
  description = "helm: Terraform installs the chart from chart_path; argocd: Terraform creates an Argo CD Application that syncs git_repo/chart_path with the values file of this environment"
  type        = string
  default     = "helm"

  validation {
    condition     = contains(["helm", "argocd"], var.delivery)
    error_message = "delivery must be helm or argocd"
  }
}

variable "chart_path" {
  description = "Path to the chart: local filesystem for helm delivery, path inside git_repo for argocd delivery"
  type        = string
  default     = "deploy/helm/rift"
}

variable "git_repo" {
  type    = string
  default = "https://github.com/rift-space/rift.git"
}

variable "git_revision" {
  type    = string
  default = "main"
}

variable "git_token" {
  description = "Token Argo CD uses to read a private git_repo; empty for public repositories"
  type        = string
  default     = ""
  sensitive   = true
}

variable "argocd_namespace" {
  type    = string
  default = "argocd"
}

variable "environment" {
  description = "Environment name; argocd delivery reads deploy/argocd/envs/<environment>/values.yaml from the repository"
  type        = string
  default     = "local"
}

variable "image" {
  type = object({
    registry    = optional(string, "ghcr.io/rift-space/rift")
    tag         = optional(string, "")
    pull_policy = optional(string, "IfNotPresent")
    pull_secret = optional(object({
      registry = string
      username = string
      password = string
    }))
  })
  default = {}
}

variable "hosts" {
  type = object({
    app     = string
    livekit = string
  })
}

variable "ingress_class" {
  type    = string
  default = "nginx"
}

variable "cluster_issuer" {
  type    = string
  default = ""
}

variable "cluster_domain" {
  description = "Cluster DNS domain; the gateway resolves its backends and LiveKit posts webhooks through fully qualified names"
  type        = string
  default     = "cluster.local"
}

variable "trusted_proxies" {
  description = "CIDRs whose X-Forwarded-For the gateway trusts: the ingress-controller pods (or nodes when it uses host networking), never whole private ranges"
  type        = list(string)
  default     = []
}

variable "ingress_namespace" {
  type    = string
  default = "ingress-nginx"
}

variable "postgres" {
  type = object({
    host           = string
    port           = number
    admin_user     = string
    admin_password = string
    bootstrap      = bool
    options        = optional(string, "")
  })
  sensitive = true
}

variable "redis_url" {
  type = string
}

variable "nats_url" {
  type = string
}

variable "minio" {
  type = object({
    endpoint = string
    bucket   = string
    user     = string
    password = string
  })
  sensitive = true
}

variable "livekit" {
  description = "namespace is the one LiveKit runs in and node_cidrs the addresses its host-networked pods send from; both are let through to voice-service for webhooks"
  type = object({
    url        = string
    client_url = string
    api_key    = string
    api_secret = string
    namespace  = optional(string, "")
    node_cidrs = optional(list(string), [])
  })
  sensitive = true
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

variable "oauth" {
  type = object({
    google_client_id     = optional(string, "")
    github_client_id     = optional(string, "")
    github_client_secret = optional(string, "")
    steam_api_key        = optional(string, "")
  })
  default   = {}
  sensitive = true
}

variable "values" {
  description = "Extra chart values merged on top of what this module derives; maps are merged key by key down to four levels (for example gateway.ingress.annotations adds to the generated annotations), lists and scalars replace the generated value"
  type        = any
  default     = {}
}
