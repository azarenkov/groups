resource "kubernetes_namespace_v1" "livekit" {
  count = var.livekit.enabled ? 1 : 0

  metadata {
    name = "livekit"
  }
}

resource "helm_release" "livekit" {
  count = var.livekit.enabled ? 1 : 0

  name       = "livekit"
  namespace  = kubernetes_namespace_v1.livekit[0].metadata[0].name
  repository = "https://helm.livekit.io"
  chart      = "livekit-server"
  version    = var.livekit.chart_version
  wait       = true
  timeout    = 600

  values = [
    yamlencode(merge({
      replicaCount = var.livekit.replicas
      livekit = merge({
        port      = 7880
        log_level = "info"
        rtc = {
          use_external_ip  = false
          port_range_start = var.livekit.udp_port
          port_range_end   = var.livekit.udp_port + 20
          tcp_port         = var.livekit.rtc_port
          node_ip          = var.livekit.node_ip
        }
        keys = {
          (var.livekit.api_key) = var.livekit.api_secret
        }
        turn = {
          enabled = false
        }
        }, var.livekit.redis_url == "" ? {} : {
        redis = {
          address = var.livekit.redis_url
        }
      })
      loadBalancer = {
        type = "disable"
      }
      deploymentStrategy = {
        type = "Recreate"
      }
      terminationGracePeriodSeconds = 30
    }, var.livekit.values))
  ]
}
