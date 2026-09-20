resource "kubernetes_namespace_v1" "monitoring" {
  count = var.monitoring.enabled ? 1 : 0

  metadata {
    name = "monitoring"
  }
}

resource "helm_release" "prometheus_operator_crds" {
  count = var.monitoring.enabled ? 1 : 0

  name       = "prometheus-operator-crds"
  namespace  = kubernetes_namespace_v1.monitoring[0].metadata[0].name
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "prometheus-operator-crds"
  version    = var.monitoring.crds_version
  wait       = true
  timeout    = 300
}

resource "helm_release" "kube_prometheus_stack" {
  count = var.monitoring.enabled ? 1 : 0

  name       = "monitoring"
  namespace  = kubernetes_namespace_v1.monitoring[0].metadata[0].name
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "kube-prometheus-stack"
  version    = var.monitoring.chart_version
  wait       = true
  timeout    = 900

  values = [
    yamlencode(merge({
      grafana = {
        ingress = {
          enabled          = var.monitoring.host != ""
          ingressClassName = var.ingress_class
          hosts            = var.monitoring.host == "" ? [] : [var.monitoring.host]
          annotations = {
            "cert-manager.io/cluster-issuer" = var.cert_manager.issuer
          }
          tls = var.monitoring.host == "" ? [] : [{
            hosts      = [var.monitoring.host]
            secretName = "grafana-tls"
          }]
        }
      }
      crds = {
        enabled = false
      }
      prometheus = {
        prometheusSpec = {
          serviceMonitorSelectorNilUsesHelmValues = false
          podMonitorSelectorNilUsesHelmValues     = false
          retention                               = "7d"
        }
      }
    }, var.monitoring.values))
  ]

  depends_on = [helm_release.prometheus_operator_crds, helm_release.ingress_nginx, helm_release.cert_manager]
}
