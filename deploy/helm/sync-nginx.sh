#!/usr/bin/env sh
set -eu
root="$(cd "$(dirname "$0")/../.." && pwd)"
sed \
  -e 's|resolver 127.0.0.11 valid=10s ipv6=off;|resolver ${NGINX_RESOLVER} valid=10s ipv6=off;|' \
  -e 's|^worker_processes auto;|worker_processes auto;\npid /tmp/nginx.pid;|' \
  -e 's|^http {|http {\n    client_body_temp_path /tmp/client_temp;\n    proxy_temp_path /tmp/proxy_temp;\n    fastcgi_temp_path /tmp/fastcgi_temp;\n    uwsgi_temp_path /tmp/uwsgi_temp;\n    scgi_temp_path /tmp/scgi_temp;|' \
  -e 's|access_log /var/log/nginx/access.log main;|access_log /dev/stdout main;|' \
  -e 's|error_log  /var/log/nginx/error.log warn;|error_log /dev/stderr warn;|' \
  -e 's|listen 80 default_server;|listen 8080 default_server;|' \
  "$root/docker/nginx/nginx.conf" > "$root/deploy/helm/rift/files/nginx.conf.template"
echo "deploy/helm/rift/files/nginx.conf.template regenerated from docker/nginx/nginx.conf"
