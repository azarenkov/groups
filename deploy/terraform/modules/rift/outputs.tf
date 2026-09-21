output "namespace" {
  value = local.namespace
}

output "voice_webhook_url" {
  value = "http://voice-service.${local.namespace}.svc.${var.cluster_domain}:8080/api/webhook/livekit"
}

output "app_url" {
  value = local.app_url
}

