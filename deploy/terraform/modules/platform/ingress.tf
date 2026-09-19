resource "kubernetes_namespace_v1" "ingress_nginx" {
  count = var.ingress_nginx.enabled ? 1 : 0

  metadata {
    name = "ingress-nginx"
  }
}

resource "helm_release" "ingress_nginx" {
  count = var.ingress_nginx.enabled ? 1 : 0

  name       = "ingress-nginx"
  namespace  = kubernetes_namespace_v1.ingress_nginx[0].metadata[0].name
  repository = "https://kubernetes.github.io/ingress-nginx"
  chart      = "ingress-nginx"
  version    = var.ingress_nginx.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode(merge({
      controller = {
        replicaCount = var.ingress_nginx.replicas
        ingressClassResource = {
          name    = var.ingress_class
          default = true
        }
        hostPort = {
          enabled = var.ingress_nginx.host_ports
        }
        service = {
          type = var.ingress_nginx.service_type
        }
        config = {
          use-forwarded-headers     = "true"
          proxy-body-size           = "512m"
          proxy-read-timeout        = "3600"
          proxy-send-timeout        = "3600"
          proxy-request-buffering   = "off"
          use-proxy-protocol        = "false"
          enable-real-ip            = "true"
          allow-snippet-annotations = "false"
        }
        metrics = {
          enabled = var.monitoring.enabled
          serviceMonitor = {
            enabled = var.monitoring.enabled
          }
        }
        admissionWebhooks = {
          enabled = true
        }
        allowSnippetAnnotations = false
      }
    }, var.ingress_nginx.values))
  ]
}
