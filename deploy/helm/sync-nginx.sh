#!/usr/bin/env sh
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sed -E \
  -e 's|resolver 127.0.0.11 valid=10s ipv6=off;|resolver ${NGINX_RESOLVER} valid=10s ipv6=off;|' \
  -e 's|^    resolver_timeout 5s;|    resolver_timeout 5s;\n    __NGINX_REAL_IP__|' \
  -e 's|^worker_processes auto;|worker_processes auto;\npid /tmp/nginx.pid;|' \
  -e 's|^http \{|http {\n    client_body_temp_path /tmp/client_temp;\n    proxy_temp_path /tmp/proxy_temp;\n    fastcgi_temp_path /tmp/fastcgi_temp;\n    uwsgi_temp_path /tmp/uwsgi_temp;\n    scgi_temp_path /tmp/scgi_temp;|' \
  -e 's|access_log /var/log/nginx/access.log main;|access_log /dev/stdout main;|' \
  -e 's|error_log  /var/log/nginx/error.log warn;|error_log /dev/stderr warn;|' \
  -e 's|listen 80 default_server;|listen 8080 default_server;|' \
  -e 's|proxy_set_header X-Forwarded-For   \$remote_addr;|proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;|' \
  -e 's|http://(identity-service\|media-service\|messaging-service\|voice-service\|realtime\|livekit):|http://\1.${NGINX_NAMESPACE}.svc.${NGINX_CLUSTER_DOMAIN}:|g' \
  "$root/docker/nginx/nginx.conf" > "$root/deploy/helm/rift/files/nginx.conf.template"
echo "deploy/helm/rift/files/nginx.conf.template regenerated from docker/nginx/nginx.conf"
