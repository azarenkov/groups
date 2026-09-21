locals {
  livekit_udp_ports = range(var.livekit_udp_port, var.livekit_udp_port + 21)
}

resource "kind_cluster" "this" {
  name            = var.cluster_name
  node_image      = "kindest/node:v1.32.2"
  wait_for_ready  = true
  kubeconfig_path = pathexpand("~/.kube/rift-local.yaml")

  kind_config {
    kind        = "Cluster"
    api_version = "kind.x-k8s.io/v1alpha4"

    networking {
      disable_default_cni = true
      pod_subnet          = var.pod_subnet
    }

    node {
      role = "control-plane"

      kubeadm_config_patches = [
        <<-EOT
        kind: InitConfiguration
        nodeRegistration:
          kubeletExtraArgs:
            node-labels: "ingress-ready=true,rift.dev/edge=true"
        EOT
      ]

      extra_port_mappings {
        container_port = 80
        host_port      = 80
        protocol       = "TCP"
      }

      extra_port_mappings {
        container_port = 443
        host_port      = 443
        protocol       = "TCP"
      }

      extra_port_mappings {
        container_port = var.livekit_tcp_port
        host_port      = var.livekit_tcp_port
        protocol       = "TCP"
      }

      dynamic "extra_port_mappings" {
        for_each = local.livekit_udp_ports
        content {
          container_port = extra_port_mappings.value
          host_port      = extra_port_mappings.value
          protocol       = "UDP"
        }
      }
    }

    dynamic "node" {
      for_each = range(var.workers)
      content {
        role = "worker"
      }
    }
  }
}

data "external" "kind_network" {
  program = ["sh", "${path.module}/kind-network.sh"]

  depends_on = [kind_cluster.this]
}

locals {
  node_cidr = var.node_cidr != "" ? var.node_cidr : data.external.kind_network.result.cidr
}

provider "kubernetes" {
  host                   = kind_cluster.this.endpoint
  client_certificate     = kind_cluster.this.client_certificate
  client_key             = kind_cluster.this.client_key
  cluster_ca_certificate = kind_cluster.this.cluster_ca_certificate
}

provider "kubectl" {
  host                   = kind_cluster.this.endpoint
  client_certificate     = kind_cluster.this.client_certificate
  client_key             = kind_cluster.this.client_key
  cluster_ca_certificate = kind_cluster.this.cluster_ca_certificate
  load_config_file       = false
}

provider "helm" {
  kubernetes = {
    host                   = kind_cluster.this.endpoint
    client_certificate     = kind_cluster.this.client_certificate
    client_key             = kind_cluster.this.client_key
    cluster_ca_certificate = kind_cluster.this.cluster_ca_certificate
  }
}

resource "helm_release" "calico" {
  name             = "calico"
  namespace        = "tigera-operator"
  create_namespace = true
  repository       = "https://docs.tigera.io/calico/charts"
  chart            = "tigera-operator"
  version          = "v3.29.3"
  wait             = true
  timeout          = 600

  values = [
    yamlencode({
      installation = {
        calicoNetwork = {
          ipPools = [{
            cidr          = var.pod_subnet
            encapsulation = "VXLAN"
          }]
        }
      }
    })
  ]

  depends_on = [kind_cluster.this]
}

resource "null_resource" "calico_ready" {
  provisioner "local-exec" {
    command = "kubectl --kubeconfig ${kind_cluster.this.kubeconfig_path} wait --for=condition=Ready nodes --all --timeout=300s"
  }

  depends_on = [helm_release.calico]
}
