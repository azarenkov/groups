output "nats_url" {
  description = "Client URL with the generated credentials; keep it in a Secret"
  value       = var.nats.enabled ? "nats://${var.nats.user}:${random_password.nats[0].result}@nats.${kubernetes_namespace_v1.nats[0].metadata[0].name}.svc.${var.cluster_domain}:4222" : ""
  sensitive   = true
}

output "livekit_url" {
  value = var.livekit.enabled ? "http://livekit-livekit-server.${kubernetes_namespace_v1.livekit[0].metadata[0].name}.svc.${var.cluster_domain}:80" : ""
}

output "livekit_namespace" {
  value = var.livekit.enabled ? kubernetes_namespace_v1.livekit[0].metadata[0].name : ""
}

output "grafana_admin_secret" {
  description = "namespace/name of the Secret with the generated Grafana admin credentials"
  value       = var.monitoring.enabled ? "${kubernetes_namespace_v1.monitoring[0].metadata[0].name}/grafana-admin" : ""
}

output "ingress_class" {
  value = var.ingress_class
}

output "cluster_issuer" {
  value = var.cert_manager.enabled ? var.cert_manager.issuer : ""
}

output "ingress_namespace" {
  value = var.ingress_nginx.enabled ? kubernetes_namespace_v1.ingress_nginx[0].metadata[0].name : ""
}

output "monitoring_crds_release" {
  description = "Identifier of the Prometheus Operator CRDs release, empty without monitoring; pass it as monitoring_ready so monitoring resources are created after the CRDs exist"
  value       = var.monitoring.enabled ? helm_release.prometheus_operator_crds[0].id : ""
}

output "argocd_release" {
  description = "Identifier of the Argo CD release; pass it as argocd_ready so Applications are created after the CRDs exist"
  value       = var.argocd.enabled ? helm_release.argocd[0].id : ""
}

output "argocd_namespace" {
  value = var.argocd.enabled ? kubernetes_namespace_v1.argocd[0].metadata[0].name : ""
}
