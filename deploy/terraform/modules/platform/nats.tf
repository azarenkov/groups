resource "kubernetes_namespace_v1" "nats" {
  count = var.nats.enabled ? 1 : 0

  metadata {
    name = "nats"
  }
}

resource "random_password" "nats" {
  count = var.nats.enabled ? 1 : 0

  length  = 40
  special = false
}

resource "kubernetes_secret_v1" "nats_auth" {
  count = var.nats.enabled ? 1 : 0

  metadata {
    name      = "nats-auth"
    namespace = kubernetes_namespace_v1.nats[0].metadata[0].name
  }

  data = {
    password = random_password.nats[0].result
  }
}

resource "helm_release" "nats" {
  count = var.nats.enabled ? 1 : 0

  name       = "nats"
  namespace  = kubernetes_namespace_v1.nats[0].metadata[0].name
  repository = "https://nats-io.github.io/k8s/helm/charts/"
  chart      = "nats"
  version    = var.nats.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode({
      container = {
        env = {
          NATS_PASSWORD = {
            valueFrom = {
              secretKeyRef = {
                name = kubernetes_secret_v1.nats_auth[0].metadata[0].name
                key  = "password"
              }
            }
          }
        }
      }
      config = {
        merge = {
          authorization = {
            user     = var.nats.user
            password = "<< $NATS_PASSWORD >>"
          }
        }
        cluster = {
          enabled  = var.nats.replicas > 1
          replicas = var.nats.replicas
        }
        jetstream = {
          enabled = true
          fileStore = {
            pvc = merge(
              { size = var.nats.storage },
              var.nats.storage_class == "" ? {} : { storageClassName = var.nats.storage_class }
            )
          }
        }
      }
      promExporter = {
        enabled = var.monitoring.enabled
        podMonitor = {
          enabled = var.monitoring.enabled
        }
      }
    })
  ]

  depends_on = [helm_release.prometheus_operator_crds, kubernetes_secret_v1.nats_auth]
}

resource "kubernetes_network_policy_v1" "nats" {
  count = var.nats.enabled ? 1 : 0

  metadata {
    name      = "nats-clients"
    namespace = kubernetes_namespace_v1.nats[0].metadata[0].name
  }

  spec {
    pod_selector {
      match_labels = {
        "app.kubernetes.io/name" = "nats"
      }
    }

    policy_types = ["Ingress"]

    ingress {
      dynamic "from" {
        for_each = var.nats.client_namespaces
        content {
          namespace_selector {
            match_labels = {
              "kubernetes.io/metadata.name" = from.value
            }
          }
        }
      }
      ports {
        port     = "4222"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = {
            "app.kubernetes.io/name" = "nats"
          }
        }
      }
    }

    dynamic "ingress" {
      for_each = var.monitoring.enabled ? [1] : []
      content {
        from {
          namespace_selector {
            match_labels = {
              "kubernetes.io/metadata.name" = "monitoring"
            }
          }
        }
        ports {
          port     = "7777"
          protocol = "TCP"
        }
      }
    }
  }
}
