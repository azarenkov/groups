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

resource "kubernetes_secret_v1" "ca" {
  count = var.cert_manager.enabled && var.cert_manager.issuer == "ca" ? 1 : 0

  metadata {
    name      = "rift-ca"
    namespace = kubernetes_namespace_v1.cert_manager[0].metadata[0].name
  }

  type = "kubernetes.io/tls"

  data = {
    "tls.crt" = var.cert_manager.ca_cert_pem
    "tls.key" = var.cert_manager.ca_key_pem
  }
}

locals {
  cluster_issuers = {
    letsencrypt = {
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
    }
    ca = {
      apiVersion = "cert-manager.io/v1"
      kind       = "ClusterIssuer"
      metadata   = { name = "ca" }
      spec       = { ca = { secretName = "rift-ca" } }
    }
    selfsigned = {
      apiVersion = "cert-manager.io/v1"
      kind       = "ClusterIssuer"
      metadata   = { name = "selfsigned" }
      spec       = { selfSigned = {} }
    }
  }
}

resource "kubectl_manifest" "cluster_issuer" {
  count = var.cert_manager.enabled ? 1 : 0

  yaml_body = yamlencode(local.cluster_issuers[var.cert_manager.issuer])

  depends_on = [helm_release.cert_manager, kubernetes_secret_v1.ca]
}
