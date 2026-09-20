SERVICES := identity-service media-service messaging-service voice-service realtime outbox-worker notification-service livekit-pooler nsfw-detector web
REGISTRY ?= rift
TAG ?= local
CLUSTER ?= rift
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
LOCAL_ENV := $(ROOT)/deploy/terraform/envs/local
KUBECONFIG_LOCAL := $(HOME)/.kube/rift-local.yaml

.PHONY: images kind-load local-init local-cluster local-up local-down smoke lint trust-ca

images:
	@for svc in $(SERVICES); do \
		echo "==> $(REGISTRY)/$$svc:$(TAG)"; \
		docker build -q -f $(ROOT)/docker/$$svc/Dockerfile -t $(REGISTRY)/$$svc:$(TAG) $(ROOT) || exit 1; \
	done

kind-load:
	@kind load docker-image --name $(CLUSTER) $(foreach svc,$(SERVICES),$(REGISTRY)/$(svc):$(TAG))

local-init:
	@cd $(LOCAL_ENV) && terraform init -input=false

local-cluster: local-init
	@cd $(LOCAL_ENV) && terraform apply -input=false -auto-approve -target=kind_cluster.this -target=helm_release.calico -target=null_resource.calico_ready

local-up: local-cluster kind-load
	@cd $(LOCAL_ENV) && terraform apply -input=false -auto-approve
	@$(MAKE) -s smoke

local-down:
	@cd $(LOCAL_ENV) && terraform destroy -input=false -auto-approve

trust-ca:
	@sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain $(LOCAL_ENV)/rift-local-ca.crt && echo "Rift local CA trusted; restart the browser"

smoke:
	@KUBECONFIG=$(KUBECONFIG_LOCAL) $(ROOT)/deploy/scripts/smoke.sh

lint:
	@helm lint $(ROOT)/deploy/helm/rift -f $(ROOT)/deploy/helm/rift/ci/lint-values.yaml
	@helm template rift $(ROOT)/deploy/helm/rift -f $(ROOT)/deploy/helm/rift/ci/lint-values.yaml > /dev/null
	@cd $(ROOT)/deploy/terraform && terraform fmt -recursive -check
	@for env in local; do (cd $(ROOT)/deploy/terraform/envs/$$env && terraform init -backend=false -input=false >/dev/null && terraform validate); done
