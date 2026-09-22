#!/bin/sh
set -eu

READY_TIMEOUT=${READY_TIMEOUT:-600}

WANTED="$K3S_VERSION $POD_SUBNET $SERVICE_SUBNET $CLUSTER_DOMAIN"
WANTED_NETWORK="$POD_SUBNET $SERVICE_SUBNET $CLUSTER_DOMAIN"
STAMP=/var/lib/rancher/k3s-rift-config

if command -v k3s >/dev/null 2>&1 && [ -f /etc/rancher/k3s/k3s.yaml ] && [ "$(cat "$STAMP" 2>/dev/null)" = "$WANTED" ]; then
    echo "k3s already installed with the requested configuration"
else
    if command -v k3s >/dev/null 2>&1; then
        installed="$(cat "$STAMP" 2>/dev/null || echo 'unknown')"
        installed_version="${installed%% *}"
        installed_network="${installed#* }"
        if [ "$installed" = "unknown" ] || [ "$installed_network" != "$WANTED_NETWORK" ]; then
            echo "k3s is installed with [$installed] but [$WANTED] was requested" >&2
            echo "the cluster CIDRs and the DNS domain cannot be changed in place: existing Services, pods and Calico keep the old ranges" >&2
            echo "run /usr/local/bin/k3s-uninstall.sh on the server (this destroys its data) and apply again" >&2
            exit 1
        fi
        if [ "$installed_version" = "$K3S_VERSION" ]; then
            echo "k3s $K3S_VERSION is installed but the stamp is stale; rewriting it" >&2
        else
            echo "upgrading k3s from $installed_version to $K3S_VERSION" >&2
        fi
    fi
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq >/dev/null
    apt-get install -y -qq curl ca-certificates open-iscsi nfs-common >/dev/null 2>&1
    swapoff -a
    sed -i '/ swap / s/^/#/' /etc/fstab
    modprobe br_netfilter
    echo br_netfilter > /etc/modules-load.d/k8s.conf
    printf 'net.bridge.bridge-nf-call-iptables=1\nnet.ipv4.ip_forward=1\n' > /etc/sysctl.d/99-k8s.conf
    sysctl --system >/dev/null
    curl -sfL https://get.k3s.io -o /tmp/get-k3s.sh
    INSTALL_K3S_VERSION="$K3S_VERSION" sh /tmp/get-k3s.sh server \
        --disable traefik --disable servicelb \
        --flannel-backend=none --disable-network-policy \
        --cluster-cidr "$POD_SUBNET" --service-cidr "$SERVICE_SUBNET" \
        --cluster-domain "$CLUSTER_DOMAIN" \
        --node-external-ip "$SERVER_IP" \
        --tls-san "$SERVER_IP" --tls-san "$APP_HOST" \
        --write-kubeconfig-mode 600 >/dev/null 2>&1
    mkdir -p "$(dirname "$STAMP")"
    printf '%s' "$WANTED" > "$STAMP"
fi

export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
kubectl create namespace tigera-operator --dry-run=client -o yaml | kubectl apply -f - >/dev/null
curl -sfL https://raw.githubusercontent.com/projectcalico/calico/v3.29.3/manifests/tigera-operator.yaml -o /tmp/tigera-operator.yaml
kubectl apply --server-side -f /tmp/tigera-operator.yaml >/dev/null
waited=0
until kubectl -n tigera-operator get deploy tigera-operator -o jsonpath='{.status.availableReplicas}' 2>/dev/null | grep -q 1; do
    waited=$((waited + 3))
    [ "$waited" -ge "$READY_TIMEOUT" ] && { echo "the Tigera operator did not become available within ${READY_TIMEOUT}s" >&2; exit 1; }
    sleep 3
done
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
waited=0
until kubectl get nodes 2>/dev/null | grep -q ' Ready'; do
    waited=$((waited + 5))
    [ "$waited" -ge "$READY_TIMEOUT" ] && { echo "the node did not become Ready within ${READY_TIMEOUT}s" >&2; exit 1; }
    sleep 5
done

for ns in kube-system calico-system tigera-operator; do
    kubectl get namespace "$ns" >/dev/null 2>&1 || continue
    kubectl -n "$ns" wait --for=condition=Ready pods --all --timeout="${READY_TIMEOUT}s" >/dev/null 2>&1 ||
        kubectl -n "$ns" wait --for=condition=Ready pods --all --timeout="${READY_TIMEOUT}s" >/dev/null ||
        { echo "cluster components in $ns did not become ready within ${READY_TIMEOUT}s" >&2; exit 1; }
done
echo "cluster ready"
