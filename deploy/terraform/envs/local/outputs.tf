output "kubeconfig" {
  value = kind_cluster.this.kubeconfig_path
}

output "app_url" {
  value = module.rift.app_url
}

output "argocd_url" {
  value = var.delivery == "argocd" ? "https://${var.argocd_host}" : ""
}
