#!/bin/sh
set -eu
cidr="$(docker network inspect kind --format '{{range .IPAM.Config}}{{println .Subnet}}{{end}}' | grep -v ':' | head -n1)"
if [ -z "$cidr" ]; then
    echo "the docker network named kind has no IPv4 subnet" >&2
    exit 1
fi
printf '{"cidr":"%s"}\n' "$cidr"
