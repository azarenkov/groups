resource "kubernetes_namespace_v1" "data" {
  metadata {
    name = var.namespace
  }
}

resource "random_password" "postgres_admin" {
  length  = 32
  special = false
}

resource "random_password" "minio_root" {
  length  = 32
  special = false
}

locals {
  storage_class = { for name, sc in {
    postgres = var.postgres.storage_class
    redis    = var.redis.storage_class
    minio    = var.minio.storage_class
  } : name => sc if sc != "" }
}

resource "helm_release" "postgres" {
  name       = "postgres"
  namespace  = kubernetes_namespace_v1.data.metadata[0].name
  repository = "oci://registry-1.docker.io/bitnamicharts"
  chart      = "postgresql"
  version    = var.postgres.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode({
      auth = {
        postgresPassword = random_password.postgres_admin.result
      }
      primary = {
        persistence = merge(
          { size = var.postgres.storage },
          contains(keys(local.storage_class), "postgres") ? { storageClass = local.storage_class.postgres } : {}
        )
        resources             = var.postgres.resources
        extendedConfiguration = "max_connections = 200\n"
      }
      metrics = {
        enabled = var.monitoring
        serviceMonitor = {
          enabled = var.monitoring
        }
      }
    })
  ]
}

resource "helm_release" "redis" {
  name       = "redis"
  namespace  = kubernetes_namespace_v1.data.metadata[0].name
  repository = "oci://registry-1.docker.io/bitnamicharts"
  chart      = "redis"
  version    = var.redis.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode({
      architecture = "standalone"
      auth = {
        enabled = false
      }
      master = {
        persistence = merge(
          { size = var.redis.storage },
          contains(keys(local.storage_class), "redis") ? { storageClass = local.storage_class.redis } : {}
        )
      }
      metrics = {
        enabled = var.monitoring
        serviceMonitor = {
          enabled = var.monitoring
        }
      }
    })
  ]
}

resource "helm_release" "minio" {
  name       = "minio"
  namespace  = kubernetes_namespace_v1.data.metadata[0].name
  repository = "oci://registry-1.docker.io/bitnamicharts"
  chart      = "minio"
  version    = var.minio.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode({
      mode = "standalone"
      auth = {
        rootUser     = "rift"
        rootPassword = random_password.minio_root.result
      }
      defaultBuckets = var.minio.bucket
      persistence = merge(
        { size = var.minio.storage },
        contains(keys(local.storage_class), "minio") ? { storageClass = local.storage_class.minio } : {}
      )
      metrics = {
        serviceMonitor = {
          enabled = var.monitoring
        }
      }
    })
  ]
}
