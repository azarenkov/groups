output "postgres" {
  value = {
    host           = "postgres.${local.namespace}.svc"
    port           = 5432
    admin_user     = "postgres"
    admin_password = random_password.postgres_admin.result
    bootstrap      = true
  }
  sensitive = true
}

output "redis_host" {
  value = "redis.${local.namespace}.svc:6379"
}

output "redis_url" {
  value = "redis://redis.${local.namespace}.svc:6379"
}

output "minio" {
  value = {
    endpoint = "http://minio.${local.namespace}.svc:9000"
    bucket   = var.minio.bucket
    user     = "rift"
    password = random_password.minio_root.result
  }
  sensitive = true
}

output "namespace" {
  value = local.namespace
}
