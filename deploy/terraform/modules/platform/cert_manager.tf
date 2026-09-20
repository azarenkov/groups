resource "kubernetes_namespace_v1" "cert_manager" {
  count = var.cert_manager.enabled ? 1 : 0

  metadata {
    name = "cert-manager"
  }
}

resource "helm_release" "cert_manager" {
  count = var.cert_manager.enabled ? 1 : 0

  name       = "cert-manager"
  namespace  = kubernetes_namespace_v1.cert_manager[0].metadata[0].name
  repository = "https://charts.jetstack.io"
  chart      = "cert-manager"
  version    = var.cert_manager.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode({
      crds = {
        enabled = true
      }
      prometheus = {
        enabled = var.monitoring.enabled
        servicemonitor = {
          enabled = var.monitoring.enabled
        }
      }
    })
  ]

  depends_on = [helm_release.prometheus_operator_crds]
}

resource "kubectl_manifest" "cluster_issuer" {
  count = var.cert_manager.enabled ? 1 : 0

  yaml_body = var.cert_manager.issuer == "letsencrypt" ? yamlencode({
    apiVersion = "cert-manager.io/v1"
    kind       = "ClusterIssuer"
    metadata   = { name = "letsencrypt" }
    spec = {
      acme = {
        server              = "https://acme-v02.api.letsencrypt.org/directory"
        email               = var.cert_manager.acme_email
        privateKeySecretRef = { name = "letsencrypt-account-key" }
        solvers = [{
          http01 = { ingress = { ingressClassName = var.ingress_class } }
        }]
      }
    }
    }) : yamlencode({
    apiVersion = "cert-manager.io/v1"
    kind       = "ClusterIssuer"
    metadata   = { name = "selfsigned" }
    spec       = { selfSigned = {} }
  })

  depends_on = [helm_release.cert_manager]
}
