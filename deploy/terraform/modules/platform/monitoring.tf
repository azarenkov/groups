resource "kubernetes_namespace_v1" "monitoring" {
  count = var.monitoring.enabled ? 1 : 0

  metadata {
    name = "monitoring"
  }
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
      prometheus = {
        prometheusSpec = {
          serviceMonitorSelectorNilUsesHelmValues = false
          podMonitorSelectorNilUsesHelmValues     = false
          retention                               = "7d"
        }
      }
    }, var.monitoring.values))
  ]

  depends_on = [helm_release.ingress_nginx]
}
