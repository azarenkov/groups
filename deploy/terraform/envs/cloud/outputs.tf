output "kubeconfig" {
  value = local_sensitive_file.kubeconfig.filename
}

output "argocd_url" {
  value = var.delivery == "argocd" ? "https://${var.argocd_host}" : ""
}

output "app_url" {
  value = module.rift.app_url
}
