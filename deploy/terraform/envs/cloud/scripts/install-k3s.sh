#!/bin/sh
set -eu

if command -v k3s >/dev/null 2>&1 && [ -f /etc/rancher/k3s/k3s.yaml ]; then
    echo "k3s already installed"
else
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq >/dev/null
    apt-get install -y -qq curl ca-certificates open-iscsi nfs-common >/dev/null 2>&1
    swapoff -a
    sed -i '/ swap / s/^/#/' /etc/fstab
    modprobe br_netfilter
    echo br_netfilter > /etc/modules-load.d/k8s.conf
    printf 'net.bridge.bridge-nf-call-iptables=1\nnet.ipv4.ip_forward=1\n' > /etc/sysctl.d/99-k8s.conf
    sysctl --system >/dev/null
    curl -sfL https://get.k3s.io | INSTALL_K3S_VERSION="$K3S_VERSION" sh -s - server \
        --disable traefik --disable servicelb \
        --flannel-backend=none --disable-network-policy \
        --cluster-cidr "$POD_SUBNET" --service-cidr "$SERVICE_SUBNET" \
        --node-external-ip "$SERVER_IP" \
        --tls-san "$SERVER_IP" --tls-san "$APP_HOST" \
        --write-kubeconfig-mode 600 >/dev/null 2>&1
fi

export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
kubectl create namespace tigera-operator --dry-run=client -o yaml | kubectl apply -f - >/dev/null
curl -sfL https://raw.githubusercontent.com/projectcalico/calico/v3.29.3/manifests/tigera-operator.yaml | kubectl apply --server-side -f - >/dev/null
until kubectl -n tigera-operator get deploy tigera-operator -o jsonpath='{.status.availableReplicas}' 2>/dev/null | grep -q 1; do sleep 3; done
cat <<CAL | kubectl apply -f - >/dev/null
apiVersion: operator.tigera.io/v1
kind: Installation
metadata:
  name: default
spec:
  calicoNetwork:
    ipPools:
      - cidr: $POD_SUBNET
        encapsulation: VXLAN
        natOutgoing: Enabled
CAL
until kubectl get nodes 2>/dev/null | grep -q ' Ready'; do sleep 5; done
until [ "$(kubectl get pods -A --no-headers | grep -vcE 'Running|Completed')" = "0" ]; do sleep 5; done
echo "cluster ready"
