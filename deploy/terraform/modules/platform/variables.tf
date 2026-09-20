variable "ingress_nginx" {
  description = "ingress-nginx controller settings; forwarded_headers and real_ip_cidrs are only for a load balancer in front of the controller that sets X-Forwarded-For"
  type = object({
    enabled           = optional(bool, true)
    chart_version     = optional(string, "4.12.1")
    host_ports        = optional(bool, false)
    service_type      = optional(string, "LoadBalancer")
    forwarded_headers = optional(bool, false)
    real_ip_cidrs     = optional(list(string), [])
    replicas          = optional(number, 1)
    values            = optional(any, {})
  })
  default = {}
}

variable "cert_manager" {
  description = "cert-manager and the ClusterIssuer the Rift ingress uses"
  type = object({
    enabled       = optional(bool, true)
    chart_version = optional(string, "v1.17.1")
    issuer        = optional(string, "selfsigned")
    acme_email    = optional(string, "")
  })
  default = {}
}

variable "nats" {
  description = "NATS JetStream cluster"
  type = object({
    enabled       = optional(bool, true)
    chart_version = optional(string, "1.3.3")
    replicas      = optional(number, 1)
    storage       = optional(string, "20Gi")
    storage_class = optional(string, "")
  })
  default = {}
}

variable "livekit" {
  description = "LiveKit SFU; with host_network the pod binds rtc_port and 21 UDP ports from udp_port on the node, without it the chart publishes rtc_port and a single UDP mux port udp_port through hostPort"
  type = object({
    enabled       = optional(bool, true)
    chart_version = optional(string, "1.8.0")
    replicas      = optional(number, 1)
    api_key       = string
    api_secret    = string
    redis_url     = optional(string, "")
    host_network  = optional(bool, true)
    rtc_port      = optional(number, 7881)
    udp_port      = optional(number, 7882)
    node_ip       = optional(string, "")
    webhook_urls  = optional(list(string), [])
    values        = optional(any, {})
  })
}

variable "argocd" {
  description = "Argo CD for GitOps delivery of the Rift release"
  type = object({
    enabled       = optional(bool, true)
    chart_version = optional(string, "7.8.26")
    host          = optional(string, "")
    values        = optional(any, {})
  })
  default = {}
}

variable "monitoring" {
  description = "kube-prometheus-stack; its CRDs are installed first so the other releases can ship ServiceMonitors"
  type = object({
    enabled       = optional(bool, false)
    chart_version = optional(string, "70.4.1")
    crds_version  = optional(string, "19.1.0")
    host          = optional(string, "")
    values        = optional(any, {})
  })
  default = {}
}

variable "cluster_domain" {
  description = "Cluster DNS domain used to build fully qualified service names"
  type        = string
  default     = "cluster.local"
}

variable "ingress_class" {
  description = "Ingress class name that ingress resources should use"
  type        = string
  default     = "nginx"
}
