output "postgres" {
  value = {
    host           = "postgres-postgresql.${kubernetes_namespace_v1.data.metadata[0].name}.svc"
    port           = 5432
    admin_user     = "postgres"
    admin_password = random_password.postgres_admin.result
    bootstrap      = true
  }
  sensitive = true
}

output "redis_url" {
  value = "redis://redis-master.${kubernetes_namespace_v1.data.metadata[0].name}.svc:6379"
}

output "minio" {
  value = {
    endpoint = "http://minio.${kubernetes_namespace_v1.data.metadata[0].name}.svc:9000"
    bucket   = var.minio.bucket
    user     = "rift"
    password = random_password.minio_root.result
  }
  sensitive = true
}

output "namespace" {
  value = kubernetes_namespace_v1.data.metadata[0].name
}
