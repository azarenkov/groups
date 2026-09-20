variable "cluster_name" {
  type    = string
  default = "rift"
}

variable "pod_subnet" {
  description = "Pod CIDR of the kind cluster; only ingress-nginx pods can reach the gateway, so this is what it trusts for X-Forwarded-For"
  type        = string
  default     = "10.244.0.0/16"
}

variable "workers" {
  type    = number
  default = 2
}

variable "app_host" {
  type    = string
  default = "rift.localtest.me"
}

variable "livekit_host" {
  type    = string
  default = "livekit.localtest.me"
}

variable "argocd_host" {
  type    = string
  default = "argocd.localtest.me"
}

variable "grafana_host" {
  type    = string
  default = "grafana.localtest.me"
}

variable "image_registry" {
  description = "Registry the Rift images come from; the Makefile loads locally built images as rift/<service>:local"
  type        = string
  default     = "rift"
}

variable "image_tag" {
  type    = string
  default = "local"
}

variable "ghcr_token" {
  description = "read:packages token for pulling private images from ghcr.io; leave empty when images are loaded into kind"
  type        = string
  default     = ""
  sensitive   = true
}

variable "ghcr_user" {
  type    = string
  default = ""
}

variable "delivery" {
  type    = string
  default = "helm"
}

variable "git_token" {
  type      = string
  default   = ""
  sensitive = true
}

variable "git_revision" {
  type    = string
  default = "main"
}

variable "monitoring" {
  type    = bool
  default = false
}

variable "livekit_tcp_port" {
  description = "Host and container port for LiveKit RTC over TCP; pick a free one when docker compose already uses 7881"
  type        = number
  default     = 7881
}

variable "livekit_udp_port" {
  description = "First of 21 UDP ports LiveKit advertises and kind publishes on the host"
  type        = number
  default     = 7882
}
