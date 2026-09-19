variable "namespace" {
  description = "Namespace that holds the self-hosted datastores"
  type        = string
  default     = "rift-data"
}

variable "storage_class" {
  description = "StorageClass for every volume; empty uses the cluster default"
  type        = string
  default     = ""
}

variable "postgres" {
  type = object({
    image   = optional(string, "postgres:16-alpine")
    storage = optional(string, "10Gi")
    resources = optional(any, {
      requests = { cpu = "250m", memory = "512Mi" }
      limits   = { memory = "1Gi" }
    })
    max_connections = optional(number, 200)
  })
  default = {}
}

variable "redis" {
  type = object({
    image   = optional(string, "redis:7-alpine")
    storage = optional(string, "2Gi")
    resources = optional(any, {
      requests = { cpu = "50m", memory = "64Mi" }
      limits   = { memory = "256Mi" }
    })
  })
  default = {}
}

variable "minio" {
  type = object({
    image   = optional(string, "quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z")
    storage = optional(string, "20Gi")
    bucket  = optional(string, "rift-attachments")
    resources = optional(any, {
      requests = { cpu = "100m", memory = "256Mi" }
      limits   = { memory = "1Gi" }
    })
  })
  default = {}
}
