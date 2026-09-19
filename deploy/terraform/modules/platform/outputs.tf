output "nats_url" {
  value = var.nats.enabled ? "nats://nats.${kubernetes_namespace_v1.nats[0].metadata[0].name}.svc:4222" : ""
}

output "livekit_url" {
  value = var.livekit.enabled ? "http://livekit-livekit-server.${kubernetes_namespace_v1.livekit[0].metadata[0].name}.svc:80" : ""
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

output "argocd_namespace" {
  value = var.argocd.enabled ? kubernetes_namespace_v1.argocd[0].metadata[0].name : ""
}
