resource "kubernetes_namespace_v1" "nats" {
  count = var.nats.enabled ? 1 : 0

  metadata {
    name = "nats"
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
      config = {
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

  depends_on = [helm_release.prometheus_operator_crds]
}
