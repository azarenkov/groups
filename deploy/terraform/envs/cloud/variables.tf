variable "server_ip" {
  description = "Public IPv4 of the VPS that runs the single-node k3s cluster"
  type        = string
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

variable "monitoring" {
  type    = bool
  default = false
}

variable "google_client_id" {
  type    = string
  default = ""
}

variable "github_client_id" {
  type    = string
  default = ""
}

variable "github_client_secret" {
  type      = string
  default   = ""
  sensitive = true
}

variable "steam_api_key" {
  type      = string
  default   = ""
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
