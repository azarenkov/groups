output "kubeconfig" {
  value = local_sensitive_file.kubeconfig.filename
}

output "app_url" {
  value = module.rift.app_url
}
