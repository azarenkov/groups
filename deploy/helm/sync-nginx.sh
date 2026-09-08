#!/usr/bin/env sh
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sed 's|resolver 127.0.0.11 valid=10s ipv6=off;|resolver ${NGINX_RESOLVER} valid=10s ipv6=off;|' \
  "$root/docker/nginx/nginx.conf" > "$root/deploy/helm/rift/files/nginx.conf.template"
echo "deploy/helm/rift/files/nginx.conf.template regenerated from docker/nginx/nginx.conf"
