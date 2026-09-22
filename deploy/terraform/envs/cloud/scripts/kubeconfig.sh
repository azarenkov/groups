#!/bin/sh
set -eu
ip="$1"; user="$2"; key="$3"
conf="$(ssh -i "$key" -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=accept-new "$user@$ip" cat /etc/rancher/k3s/k3s.yaml | base64 | tr -d '\n')"
printf '{"kubeconfig":"%s"}\n' "$conf"
