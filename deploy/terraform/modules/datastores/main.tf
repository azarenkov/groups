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

resource "random_password" "redis" {
  length  = 32
  special = false
}

locals {
  namespace   = kubernetes_namespace_v1.data.metadata[0].name
  store_names = toset(["postgres", "redis", "minio"])
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
      args      = ["redis-server", "--appendonly", "yes", "--requirepass", "$(REDIS_PASSWORD)"]
      env = {
        REDIS_PASSWORD = random_password.redis.result
      }
      probe = ["sh", "-c", "redis-cli -a \"$REDIS_PASSWORD\" --no-auth-warning ping | grep -q PONG"]
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
  for_each = local.store_names

  metadata {
    name      = each.key
    namespace = local.namespace
  }

  data = local.stores[each.key].env
}

resource "kubernetes_stateful_set_v1" "store" {
  for_each = local.store_names

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
          run_as_user     = local.stores[each.key].uid
          run_as_group    = local.stores[each.key].uid
          fs_group        = local.stores[each.key].uid
          seccomp_profile {
            type = "RuntimeDefault"
          }
        }

        container {
          name  = each.key
          image = local.stores[each.key].image
          args  = local.stores[each.key].args

          port {
            name           = "main"
            container_port = local.stores[each.key].port
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
            requests = try(local.stores[each.key].resources.requests, {})
            limits   = try(local.stores[each.key].resources.limits, {})
          }

          startup_probe {
            exec {
              command = local.stores[each.key].probe
            }
            period_seconds    = 10
            failure_threshold = var.startup_failure_threshold
          }

          readiness_probe {
            exec {
              command = local.stores[each.key].probe
            }
            period_seconds = 5
          }

          liveness_probe {
            exec {
              command = local.stores[each.key].probe
            }
            period_seconds    = 15
            failure_threshold = 5
          }

          volume_mount {
            name       = "data"
            mount_path = local.stores[each.key].mount
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
            storage = local.stores[each.key].storage
          }
        }
      }
    }
  }

  wait_for_rollout = true
}

resource "kubernetes_service_v1" "store" {
  for_each = local.store_names

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
      port        = local.stores[each.key].port
      target_port = "main"
    }
  }
}

resource "kubernetes_network_policy_v1" "clients" {
  for_each = local.store_names

  metadata {
    name      = "${each.key}-clients"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = {
        "app.kubernetes.io/name" = each.key
      }
    }

    policy_types = ["Ingress"]

    dynamic "ingress" {
      for_each = length(concat(var.clients.namespaces, each.key == "redis" ? var.clients.redis_cidrs : [])) > 0 ? [1] : []
      content {
        dynamic "from" {
          for_each = var.clients.namespaces
          content {
            namespace_selector {
              match_labels = {
                "kubernetes.io/metadata.name" = from.value
              }
            }
          }
        }
        dynamic "from" {
          for_each = each.key == "redis" ? var.clients.redis_cidrs : []
          content {
            ip_block {
              cidr = from.value
            }
          }
        }
        ports {
          port     = tostring(local.stores[each.key].port)
          protocol = "TCP"
        }
      }
    }
  }
}
