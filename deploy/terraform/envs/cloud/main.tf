locals {
  rift_namespace    = "rift"
  livekit_namespace = "livekit"
  scheme            = "https"
}

resource "random_password" "livekit_secret" {
  length  = 48
  special = false
}

module "platform" {
  source = "../../modules/platform"

  cluster_domain = var.cluster_domain

  ingress_nginx = {
    host_ports   = true
    service_type = "ClusterIP"
  }

  cert_manager = {
    issuer     = "letsencrypt"
    acme_email = var.acme_email
  }

  nats = {
    replicas          = 1
    storage           = "5Gi"
    client_namespaces = [local.rift_namespace]
  }

  livekit = {
    namespace      = local.livekit_namespace
    api_key        = "rift-server"
    api_secret     = random_password.livekit_secret.result
    redis_url      = module.datastores.redis_host
    redis_password = module.datastores.redis_password
    rtc_port       = var.livekit_tcp_port
    udp_port       = var.livekit_udp_port
    node_ip        = var.server_ip
    webhook_urls   = ["http://voice-service.${local.rift_namespace}.svc.${var.cluster_domain}:8080/api/webhook/livekit"]
  }

  argocd = {
    enabled = var.delivery == "argocd"
    host    = var.argocd_host
  }

  monitoring = {
    enabled = var.monitoring
    host    = var.grafana_host
  }

  depends_on = [null_resource.k3s]
}

module "datastores" {
  source = "../../modules/datastores"

  cluster_domain = var.cluster_domain

  postgres = {
    storage = "8Gi"
    resources = {
      requests = { cpu = "250m", memory = "512Mi" }
      limits   = { memory = "1Gi" }
    }
  }
  redis = { storage = "1Gi" }
  minio = { storage = "8Gi" }

  clients = {
    namespaces  = [local.rift_namespace, local.livekit_namespace]
    redis_cidrs = ["${var.server_ip}/32", var.pod_subnet]
  }

  depends_on = [null_resource.k3s]
}

module "rift" {
  source = "../../modules/rift"

  namespace      = local.rift_namespace
  cluster_domain = var.cluster_domain
  delivery       = var.delivery
  environment    = "staging"
  chart_path     = var.delivery == "helm" ? "${path.module}/../../../helm/rift" : "deploy/helm/rift"
  argocd_ready   = module.platform.argocd_release
  git_revision   = var.git_revision
  git_token      = var.git_token

  image = {
    registry    = var.image_registry
    tag         = var.image_tag
    pull_policy = "IfNotPresent"
    pull_secret = var.ghcr_token == "" ? null : {
      registry = "ghcr.io"
      username = var.ghcr_user
      password = var.ghcr_token
    }
  }

  hosts = {
    app     = var.app_host
    livekit = var.livekit_host
  }

  ingress_class     = module.platform.ingress_class
  ingress_namespace = module.platform.ingress_namespace
  cluster_issuer    = module.platform.cluster_issuer
  trusted_proxies   = [var.pod_subnet]
  postgres          = module.datastores.postgres
  redis_url         = module.datastores.redis_url
  nats_url          = module.platform.nats_url
  minio             = module.datastores.minio

  livekit = {
    url        = module.platform.livekit_url
    client_url = "wss://${var.livekit_host}"
    api_key    = "rift-server"
    api_secret = random_password.livekit_secret.result
    namespace  = local.livekit_namespace
    node_cidrs = ["${var.server_ip}/32", var.pod_subnet]
  }

  mailer = var.mailer

  oauth = {
    google_client_id     = var.google_client_id
    github_client_id     = var.github_client_id
    github_client_secret = var.github_client_secret
    steam_api_key        = var.steam_api_key
  }

  values = {
    config = {
      NATS_STREAM_HOT_MAX_BYTES  = "536870912"
      NATS_STREAM_COLD_MAX_BYTES = "134217728"
    }
    httpServices = {
      identity-service  = { replicas = 1 }
      media-service     = { replicas = 1 }
      messaging-service = { replicas = 1 }
      voice-service     = { replicas = 1 }
    }
    realtime = { replicas = 1 }
    gateway = {
      replicas = 1
      ingress = {
        annotations = { "nginx.ingress.kubernetes.io/proxy-body-size" = "512m" }
      }
    }
    web = { replicas = 1 }
    nsfwDetector = {
      resources = {
        requests = { cpu = "250m", memory = "1Gi" }
        limits   = { memory = "3Gi" }
      }
    }
  }
}
