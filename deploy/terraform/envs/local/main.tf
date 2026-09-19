resource "random_password" "livekit_secret" {
  length  = 48
  special = false
}

module "platform" {
  source = "../../modules/platform"

  ingress_nginx = {
    host_ports   = true
    service_type = "NodePort"
    values = {
      controller = {
        nodeSelector = { "ingress-ready" = "true" }
        tolerations = [{
          key      = "node-role.kubernetes.io/control-plane"
          operator = "Exists"
          effect   = "NoSchedule"
        }]
      }
    }
  }

  cert_manager = {
    issuer = "selfsigned"
  }

  nats = {
    replicas = 1
    storage  = "2Gi"
  }

  livekit = {
    api_key    = "rift-server"
    api_secret = random_password.livekit_secret.result
    redis_url  = "redis-master.${module.datastores.namespace}.svc:6379"
    rtc_port   = var.livekit_tcp_port
    udp_port   = var.livekit_udp_port
    node_ip    = "127.0.0.1"
    values = {
      nodeSelector = { "rift.dev/edge" = "true" }
      tolerations = [{
        key      = "node-role.kubernetes.io/control-plane"
        operator = "Exists"
        effect   = "NoSchedule"
      }]
    }
  }

  argocd = {
    enabled = var.delivery == "argocd"
    host    = var.argocd_host
  }

  monitoring = {
    enabled = var.monitoring
    host    = var.grafana_host
  }

  depends_on = [null_resource.calico_ready]
}

module "datastores" {
  source = "../../modules/datastores"

  postgres = {
    storage = "5Gi"
    resources = {
      requests = { cpu = "100m", memory = "256Mi" }
      limits   = { memory = "1Gi" }
    }
  }
  redis = { storage = "1Gi" }
  minio = { storage = "5Gi" }

  depends_on = [null_resource.calico_ready]
}

module "rift" {
  source = "../../modules/rift"

  delivery     = var.delivery
  environment  = "local"
  chart_path   = var.delivery == "helm" ? "${path.module}/../../../helm/rift" : "deploy/helm/rift"
  git_revision = var.git_revision
  git_token    = var.git_token

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
  postgres          = module.datastores.postgres
  redis_url         = module.datastores.redis_url
  nats_url          = module.platform.nats_url
  minio             = module.datastores.minio

  livekit = {
    url        = module.platform.livekit_url
    client_url = "wss://${var.livekit_host}"
    api_key    = "rift-server"
    api_secret = random_password.livekit_secret.result
  }

  values = {
    config = {
      NATS_STREAM_HOT_MAX_BYTES  = "134217728"
      NATS_STREAM_COLD_MAX_BYTES = "33554432"
    }
    httpServices = {
      identity-service  = { replicas = 1 }
      media-service     = { replicas = 1 }
      messaging-service = { replicas = 1 }
      voice-service     = { replicas = 1 }
    }
    realtime = { replicas = 1 }
    gateway  = { replicas = 1 }
    web      = { replicas = 1 }
    nsfwDetector = {
      resources = {
        requests = { cpu = "250m", memory = "1Gi" }
        limits   = { memory = "3Gi" }
      }
    }
  }
}
