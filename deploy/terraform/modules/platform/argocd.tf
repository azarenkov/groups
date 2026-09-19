resource "kubernetes_namespace_v1" "argocd" {
  count = var.argocd.enabled ? 1 : 0

  metadata {
    name = "argocd"
  }
}

resource "helm_release" "argocd" {
  count = var.argocd.enabled ? 1 : 0

  name       = "argocd"
  namespace  = kubernetes_namespace_v1.argocd[0].metadata[0].name
  repository = "https://argoproj.github.io/argo-helm"
  chart      = "argo-cd"
  version    = var.argocd.chart_version
  wait       = true
  timeout    = 900

  values = [
    yamlencode(merge({
      configs = {
        params = {
          "server.insecure" = true
        }
      }
      server = {
        ingress = {
          enabled          = var.argocd.host != ""
          ingressClassName = var.ingress_class
          hostname         = var.argocd.host
          annotations = {
            "cert-manager.io/cluster-issuer" = var.cert_manager.issuer
          }
          tls = var.argocd.host != ""
        }
      }
      dex = {
        enabled = false
      }
      notifications = {
        enabled = false
      }
    }, var.argocd.values))
  ]

  depends_on = [helm_release.ingress_nginx, helm_release.cert_manager]
}
