variable "namespace" {
  description = "Namespace that holds the self-hosted datastores"
  type        = string
  default     = "rift-data"
}

variable "postgres" {
  type = object({
    chart_version = optional(string, "16.6.3")
    storage       = optional(string, "10Gi")
    storage_class = optional(string, "")
    resources = optional(any, {
      requests = { cpu = "250m", memory = "512Mi" }
      limits   = { memory = "1Gi" }
    })
  })
  default = {}
}

variable "redis" {
  type = object({
    chart_version = optional(string, "20.11.3")
    storage       = optional(string, "2Gi")
    storage_class = optional(string, "")
  })
  default = {}
}

variable "minio" {
  type = object({
    chart_version = optional(string, "16.0.5")
    storage       = optional(string, "20Gi")
    storage_class = optional(string, "")
    bucket        = optional(string, "rift-attachments")
  })
  default = {}
}

variable "monitoring" {
  type    = bool
  default = false
}
