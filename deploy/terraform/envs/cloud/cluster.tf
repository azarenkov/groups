locals {
  ssh_key = pathexpand(var.ssh_private_key)
}

resource "null_resource" "k3s" {
  triggers = {
    server_ip      = var.server_ip
    k3s_version    = var.k3s_version
    pod_subnet     = var.pod_subnet
    service_subnet = var.service_subnet
    cluster_domain = var.cluster_domain
    installer      = filesha256("${path.module}/scripts/install-k3s.sh")
  }

  connection {
    type        = "ssh"
    host        = var.server_ip
    user        = var.ssh_user
    private_key = file(local.ssh_key)
  }

  provisioner "file" {
    source      = "${path.module}/scripts/install-k3s.sh"
    destination = "/tmp/install-k3s.sh"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install-k3s.sh",
      "K3S_VERSION='${var.k3s_version}' POD_SUBNET='${var.pod_subnet}' SERVICE_SUBNET='${var.service_subnet}' CLUSTER_DOMAIN='${var.cluster_domain}' SERVER_IP='${var.server_ip}' APP_HOST='${var.app_host}' /tmp/install-k3s.sh",
    ]
  }
}

data "external" "kubeconfig" {
  program = ["sh", "${path.module}/scripts/kubeconfig.sh", var.server_ip, var.ssh_user, local.ssh_key]

  depends_on = [null_resource.k3s]
}

locals {
  kubeconfig      = yamldecode(base64decode(data.external.kubeconfig.result.kubeconfig))
  kube_host       = "https://${var.server_ip}:6443"
  kubeconfig_path = pathexpand(var.kubeconfig_path)
  kube_ca         = base64decode(local.kubeconfig.clusters[0].cluster["certificate-authority-data"])
  kube_cert       = base64decode(local.kubeconfig.users[0].user["client-certificate-data"])
  kube_key        = base64decode(local.kubeconfig.users[0].user["client-key-data"])
}

resource "terraform_data" "kube_dir" {
  input            = dirname(local.kubeconfig_path)
  triggers_replace = dirname(local.kubeconfig_path)

  provisioner "local-exec" {
    command = "mkdir -p '${self.input}'"
  }
}

resource "local_sensitive_file" "kubeconfig" {
  content         = replace(base64decode(data.external.kubeconfig.result.kubeconfig), "https://127.0.0.1:6443", local.kube_host)
  filename        = local.kubeconfig_path
  file_permission = "0600"

  depends_on = [terraform_data.kube_dir]
}

provider "kubernetes" {
  host                   = local.kube_host
  client_certificate     = local.kube_cert
  client_key             = local.kube_key
  cluster_ca_certificate = local.kube_ca
}

provider "kubectl" {
  host                   = local.kube_host
  client_certificate     = local.kube_cert
  client_key             = local.kube_key
  cluster_ca_certificate = local.kube_ca
  load_config_file       = false
}

provider "helm" {
  kubernetes = {
    host                   = local.kube_host
    client_certificate     = local.kube_cert
    client_key             = local.kube_key
    cluster_ca_certificate = local.kube_ca
  }
}
