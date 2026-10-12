resource "kubernetes_namespace_v1" "monitoring" {
  count = var.monitoring.enabled ? 1 : 0

  metadata {
    name = "monitoring"
  }
}

resource "random_password" "grafana_admin" {
  count = var.monitoring.enabled ? 1 : 0

  length  = 32
  special = false
}

resource "kubernetes_secret_v1" "grafana_admin" {
  count = var.monitoring.enabled ? 1 : 0

  metadata {
    name      = "grafana-admin"
    namespace = kubernetes_namespace_v1.monitoring[0].metadata[0].name
  }

  data = {
    "admin-user"     = "admin"
    "admin-password" = random_password.grafana_admin[0].result
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

  values = [yamlencode(module.monitoring_values.result)]

  depends_on = [helm_release.prometheus_operator_crds, helm_release.ingress_nginx, helm_release.cert_manager, kubernetes_secret_v1.grafana_admin]
}

module "monitoring_values" {
  source = "../deepmerge"

  base = {
    grafana = {
      admin = {
        existingSecret = "grafana-admin"
        userKey        = "admin-user"
        passwordKey    = "admin-password"
      }
      podAnnotations = {
        "checksum/admin" = var.monitoring.enabled ? sha256(random_password.grafana_admin[0].result) : ""
      }
      ingress = {
        enabled          = var.monitoring.host != ""
        ingressClassName = var.ingress_class
        hosts            = var.monitoring.host == "" ? [] : [var.monitoring.host]
        annotations      = local.ingress_tls_annotations
        tls = var.monitoring.host == "" || !var.cert_manager.enabled ? [] : [{
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
        ruleSelectorNilUsesHelmValues           = false
        serviceMonitorNamespaceSelector         = {}
        podMonitorNamespaceSelector             = {}
        ruleNamespaceSelector                   = {}
        retention                               = "7d"
      }
    }
  }
  override = var.monitoring.values
}
