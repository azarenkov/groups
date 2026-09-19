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
  namespace = kubernetes_namespace_v1.data.metadata[0].name
  stores = {
    postgres = {
      image     = var.postgres.image
      port      = 5432
      storage   = var.postgres.storage
      resources = var.postgres.resources
      mount     = "/var/lib/postgresql/data"
      uid       = 70
      args      = ["-c", "max_connections=${var.postgres.max_connections}"]
      env = {
        POSTGRES_USER     = "postgres"
        POSTGRES_PASSWORD = random_password.postgres_admin.result
        PGDATA            = "/var/lib/postgresql/data/pgdata"
      }
      probe = ["pg_isready", "-U", "postgres"]
    }
    redis = {
      image     = var.redis.image
      port      = 6379
      storage   = var.redis.storage
      resources = var.redis.resources
      mount     = "/data"
      uid       = 999
      args      = ["redis-server", "--appendonly", "yes"]
      env       = {}
      probe     = ["redis-cli", "ping"]
    }
    minio = {
      image     = var.minio.image
      port      = 9000
      storage   = var.minio.storage
      resources = var.minio.resources
      mount     = "/data"
      uid       = 1000
      args      = ["server", "/data", "--console-address", ":9001"]
      env = {
        MINIO_ROOT_USER     = "rift"
        MINIO_ROOT_PASSWORD = random_password.minio_root.result
      }
      probe = ["sh", "-c", "curl -sf http://127.0.0.1:9000/minio/health/live"]
    }
  }
}

resource "kubernetes_secret_v1" "store" {
  for_each = local.stores

  metadata {
    name      = each.key
    namespace = local.namespace
  }

  data = each.value.env
}

resource "kubernetes_stateful_set_v1" "store" {
  for_each = local.stores

  metadata {
    name      = each.key
    namespace = local.namespace
    labels = {
      "app.kubernetes.io/name"    = each.key
      "app.kubernetes.io/part-of" = "rift"
    }
  }

  spec {
    service_name = each.key
    replicas     = 1

    selector {
      match_labels = {
        "app.kubernetes.io/name" = each.key
      }
    }

    template {
      metadata {
        labels = {
          "app.kubernetes.io/name"    = each.key
          "app.kubernetes.io/part-of" = "rift"
        }
      }

      spec {
        security_context {
          run_as_non_root = true
          run_as_user     = each.value.uid
          run_as_group    = each.value.uid
          fs_group        = each.value.uid
          seccomp_profile {
            type = "RuntimeDefault"
          }
        }

        container {
          name  = each.key
          image = each.value.image
          args  = each.value.args

          port {
            name           = "main"
            container_port = each.value.port
          }

          env_from {
            secret_ref {
              name = kubernetes_secret_v1.store[each.key].metadata[0].name
            }
          }

          security_context {
            allow_privilege_escalation = false
            capabilities {
              drop = ["ALL"]
            }
          }

          resources {
            requests = try(each.value.resources.requests, {})
            limits   = try(each.value.resources.limits, {})
          }

          readiness_probe {
            exec {
              command = each.value.probe
            }
            period_seconds = 5
          }

          liveness_probe {
            exec {
              command = each.value.probe
            }
            period_seconds    = 15
            failure_threshold = 5
          }

          volume_mount {
            name       = "data"
            mount_path = each.value.mount
          }
        }
      }
    }

    volume_claim_template {
      metadata {
        name = "data"
      }

      spec {
        access_modes       = ["ReadWriteOnce"]
        storage_class_name = var.storage_class == "" ? null : var.storage_class

        resources {
          requests = {
            storage = each.value.storage
          }
        }
      }
    }
  }

  wait_for_rollout = true
}

resource "kubernetes_service_v1" "store" {
  for_each = local.stores

  metadata {
    name      = each.key
    namespace = local.namespace
    labels = {
      "app.kubernetes.io/name" = each.key
    }
  }

  spec {
    selector = {
      "app.kubernetes.io/name" = each.key
    }

    port {
      name        = "main"
      port        = each.value.port
      target_port = "main"
    }
  }
}
