output "ca_certificate" {
  description = "Trust this once and every https://*.localtest.me certificate, WebSockets included, is accepted"
  value       = local_file.ca_cert.filename
}

output "kubeconfig" {
  value = kind_cluster.this.kubeconfig_path
}

output "app_url" {
  value = module.rift.app_url
}

output "argocd_url" {
  value = var.delivery == "argocd" ? "https://${var.argocd_host}" : ""
}

