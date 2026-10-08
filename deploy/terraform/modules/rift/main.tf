resource "kubernetes_namespace_v1" "rift" {
  metadata {
    name = var.namespace
    labels = {
      "app.kubernetes.io/part-of" = "rift"
    }
  }
}

resource "random_password" "jwt" {
  length  = 64
  special = false
}

resource "random_password" "altcha" {
  length  = 64
  special = false
}

resource "random_password" "trial_hash" {
  length  = 64
  special = false
}

resource "random_password" "internal_api" {
  length  = 64
  special = false
}

resource "random_password" "database" {
  for_each = toset(local.database_services)

  length  = 32
  special = false
}

locals {
  git_ssh           = nonsensitive(var.git_ssh_key != "")
  git_credentials   = local.git_ssh || nonsensitive(var.git_token != "")
  git_repo_url      = local.git_ssh ? var.git_repo_ssh : var.git_repo
  database_services = ["identity", "media", "messaging", "voice", "realtime"]
  namespace         = kubernetes_namespace_v1.rift.metadata[0].name
  scheme            = var.cluster_issuer == "" ? "http" : "https"
  app_url           = "${local.scheme}://${var.hosts.app}"

  livekit_peers = concat(
    var.livekit.namespace == "" ? [] : [{
      namespaceSelector = {
        matchLabels = {
          "kubernetes.io/metadata.name" = var.livekit.namespace
        }
      }
    }],
    [for cidr in var.livekit.node_cidrs : { ipBlock = { cidr = cidr } }]
  )

  managed_secrets = {
    NATS_URL             = var.nats_url
    REDIS_URL            = var.redis_url
    JWT_SECRET           = random_password.jwt.result
    ALTCHA_HMAC_KEY      = random_password.altcha.result
    TRIAL_HASH_KEY       = random_password.trial_hash.result
    INTERNAL_API_TOKEN   = random_password.internal_api.result
    LIVEKIT_API_SECRET   = var.livekit.api_secret
    MINIO_ROOT_USER      = var.minio.user
    MINIO_ROOT_PASSWORD  = var.minio.password
    MAILER_USERNAME      = var.mailer.username
    MAILER_PASSWORD      = var.mailer.password
    GITHUB_CLIENT_ID     = var.oauth.github_client_id
    GITHUB_CLIENT_SECRET = var.oauth.github_client_secret
    STEAM_API_KEY        = var.oauth.steam_api_key
  }

  database_passwords = { for svc in local.database_services : "${svc}-password" => random_password.database[svc].result }

  derived_values = {
    podAnnotations = {
      "checksum/managed-secrets" = sha256(jsonencode({
        secrets   = local.managed_secrets
        databases = local.database_passwords
        registry  = var.image.pull_secret
      }))
    }
    image = {
      registry   = var.image.registry
      tag        = var.image.tag
      pullPolicy = var.image.pull_policy
    }
    imagePullSecrets = var.image.pull_secret == null ? [] : [{ name = "${var.release_name}-registry" }]
    config = {
      POSTGRES_HOST                  = var.postgres.host
      POSTGRES_PORT                  = tostring(var.postgres.port)
      POSTGRES_OPTIONS               = var.postgres.options
      LIVEKIT_URL                    = var.livekit.url
      LIVEKIT_CLIENT_URL             = var.livekit.client_url
      LIVEKIT_API_KEY                = var.livekit.api_key
      MINIO_ENDPOINT                 = var.minio.endpoint
      MINIO_BUCKET                   = var.minio.bucket
      PUBLIC_API_URL                 = local.app_url
      PUBLIC_APP_URL                 = local.app_url
      CORS_ALLOWED_ORIGINS           = "${local.app_url},file://,null"
      MAILER_HOST                    = var.mailer.host
      MAILER_PORT                    = var.mailer.port
      MAILER_FROM                    = var.mailer.from
      MAILER_PASSWORD_RESET_BASE_URL = local.app_url
    }
    secrets = {
      existingSecret = "${var.release_name}-secrets"
    }
    databases = {
      existingSecret = "${var.release_name}-databases"
      bootstrap = {
        enabled        = var.postgres.bootstrap
        adminUser      = var.postgres.admin_user
        adminDatabase  = "postgres"
        existingSecret = "${var.release_name}-postgres-admin"
      }
    }
    gateway = {
      clusterDomain   = var.cluster_domain
      livekitUpstream = var.livekit.url
      trustedProxies  = var.trusted_proxies
      ingress = {
        enabled     = true
        className   = var.ingress_class
        host        = var.hosts.app
        livekitHost = var.hosts.livekit
        annotations = var.cluster_issuer == "" ? {} : {
          "cert-manager.io/cluster-issuer" = var.cluster_issuer
        }
        tls = var.cluster_issuer == "" ? [] : [{
          hosts      = [var.hosts.app, var.hosts.livekit]
          secretName = "${var.release_name}-tls"
        }]
      }
    }
    networkPolicy = {
      enabled = true
      ingressController = {
        namespaceSelector = {
          matchLabels = {
            "kubernetes.io/metadata.name" = var.ingress_namespace
          }
        }
      }
      extraFrom = length(local.livekit_peers) == 0 ? {} : { voice-service = local.livekit_peers }
    }
  }

  chart_values = module.chart_values.result

  chart_checksum = var.delivery == "helm" ? sha256(join("", [
    for f in sort(fileset(var.chart_path, "**")) : filesha256("${var.chart_path}/${f}")
  ])) : ""

  argocd_values = merge(local.chart_values, { image = { pullPolicy = var.image.pull_policy } })
}

module "chart_values" {
  source = "../deepmerge"

  base     = local.derived_values
  override = var.values
}

resource "kubernetes_secret_v1" "secrets" {
  metadata {
    name      = "${var.release_name}-secrets"
    namespace = local.namespace
  }

  data = local.managed_secrets
}

resource "kubernetes_secret_v1" "databases" {
  metadata {
    name      = "${var.release_name}-databases"
    namespace = local.namespace
  }

  data = local.database_passwords
}

resource "kubernetes_secret_v1" "postgres_admin" {
  count = var.postgres.bootstrap ? 1 : 0

  metadata {
    name      = "${var.release_name}-postgres-admin"
    namespace = local.namespace
  }

  data = {
    "admin-password" = var.postgres.admin_password
  }
}

resource "kubernetes_secret_v1" "registry" {
  count = var.image.pull_secret == null ? 0 : 1

  metadata {
    name      = "${var.release_name}-registry"
    namespace = local.namespace
  }

  type = "kubernetes.io/dockerconfigjson"

  data = {
    ".dockerconfigjson" = jsonencode({
      auths = {
        (var.image.pull_secret.registry) = {
          username = var.image.pull_secret.username
          password = var.image.pull_secret.password
          auth     = base64encode("${var.image.pull_secret.username}:${var.image.pull_secret.password}")
        }
      }
    })
  }
}

resource "helm_release" "rift" {
  count = var.delivery == "helm" ? 1 : 0

  name      = var.release_name
  namespace = local.namespace
  chart     = var.chart_path
  wait      = true
  timeout   = 900

  values = [
    yamlencode(local.chart_values),
    yamlencode({ chartChecksum = local.chart_checksum }),
  ]

  depends_on = [
    kubernetes_secret_v1.secrets,
    kubernetes_secret_v1.databases,
    kubernetes_secret_v1.postgres_admin,
    kubernetes_secret_v1.registry,
  ]
}

resource "terraform_data" "argocd_ready" {
  count = var.delivery == "argocd" ? 1 : 0

  input = var.argocd_ready

  lifecycle {
    precondition {
      condition     = var.argocd_ready != ""
      error_message = "delivery = \"argocd\" needs argocd_ready, so the Application is created after Argo CD and its CRDs exist"
    }
  }
}

resource "kubernetes_secret_v1" "argocd_repo" {
  count = var.delivery == "argocd" && local.git_credentials ? 1 : 0

  metadata {
    name      = "${var.release_name}-repo"
    namespace = var.argocd_namespace
    labels = {
      "argocd.argoproj.io/secret-type" = "repository"
    }
  }

  data = merge(
    {
      type = "git"
      url  = local.git_repo_url
    },
    var.git_ssh_key != ""
    ? { sshPrivateKey = var.git_ssh_key }
    : { username = "git", password = var.git_token },
  )

  depends_on = [terraform_data.argocd_ready]
}

resource "kubectl_manifest" "argocd_application" {
  count = var.delivery == "argocd" ? 1 : 0

  yaml_body = yamlencode({
    apiVersion = "argoproj.io/v1alpha1"
    kind       = "Application"
    metadata = {
      name       = var.release_name
      namespace  = var.argocd_namespace
      finalizers = ["resources-finalizer.argocd.argoproj.io"]
    }
    spec = {
      project = "default"
      source = {
        repoURL        = local.git_repo_url
        targetRevision = var.git_revision
        path           = var.chart_path
        helm = {
          releaseName  = var.release_name
          valueFiles   = ["../../argocd/envs/${var.environment}/values.yaml"]
          valuesObject = local.argocd_values
        }
      }
      destination = {
        server    = "https://kubernetes.default.svc"
        namespace = local.namespace
      }
      syncPolicy = {
        automated = {
          prune    = true
          selfHeal = true
        }
        syncOptions = ["CreateNamespace=false", "ServerSideApply=true"]
      }
    }
  })

  depends_on = [
    kubernetes_secret_v1.secrets,
    kubernetes_secret_v1.databases,
    kubernetes_secret_v1.postgres_admin,
    kubernetes_secret_v1.registry,
    kubernetes_secret_v1.argocd_repo,
    terraform_data.argocd_ready,
  ]
}
