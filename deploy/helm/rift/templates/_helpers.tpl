{{- define "rift.labels" -}}
app.kubernetes.io/name: rift
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Values.image.tag | default .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version }}
{{- end }}

{{- define "rift.selectorLabels" -}}
app.kubernetes.io/name: rift
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .component }}
{{- end }}

{{- define "rift.replicas" -}}
{{- $hpa := include "rift.autoscaling" . | fromYaml -}}
{{- if not $hpa.enabled }}
replicas: {{ .workload.replicas }}
{{- end }}
{{- end }}

{{- define "rift.autoscaling" -}}
{{- $defaults := .root.Values.autoscaling -}}
{{- $own := .workload.autoscaling | default dict -}}
{{- $merged := mergeOverwrite (deepCopy $defaults) $own -}}
{{- toYaml $merged -}}
{{- end }}

{{- define "rift.image" -}}
{{- printf "%s/%s:%s" .root.Values.image.registry .service (.root.Values.image.tag | default .root.Chart.AppVersion) -}}
{{- end }}

{{- define "rift.envName" -}}
{{ .Release.Name }}-env
{{- end }}

{{- define "rift.secretName" -}}
{{ .Values.secrets.existingSecret | default (printf "%s-secrets" .Release.Name) }}
{{- end }}

{{- define "rift.podSecurityContext" -}}
runAsNonRoot: true
runAsUser: 10001
runAsGroup: 10001
fsGroup: 10001
seccompProfile:
  type: RuntimeDefault
{{- end }}

{{- define "rift.containerSecurityContext" -}}
allowPrivilegeEscalation: false
readOnlyRootFilesystem: true
capabilities:
  drop: ["ALL"]
{{- end }}

{{- define "rift.databaseSecretName" -}}
{{ .Values.databases.existingSecret | default (printf "%s-databases" .Release.Name) }}
{{- end }}

{{- define "rift.databaseEnv" -}}
{{- $root := .root -}}
{{- range .databases }}
{{- $db := index $root.Values.databases.services . }}
{{- $prefix := upper . }}
- name: {{ $prefix }}_POSTGRES_DB
  value: {{ $db.name | quote }}
- name: {{ $prefix }}_POSTGRES_USER
  value: {{ $db.user | quote }}
- name: {{ $prefix }}_POSTGRES_PASSWORD
  valueFrom:
    secretKeyRef:
      name: {{ include "rift.databaseSecretName" $root }}
      key: {{ printf "%s-password" . }}
{{- end }}
{{- end }}

{{- define "rift.envFrom" -}}
envFrom:
  - configMapRef:
      name: {{ include "rift.envName" . }}
  - secretRef:
      name: {{ include "rift.secretName" . }}
{{- end }}

{{- define "rift.checksums" -}}
checksum/env: {{ .Values.config | toYaml | sha256sum }}
checksum/secrets: {{ .Values.secrets | toYaml | sha256sum }}
checksum/databases: {{ .Values.databases | toYaml | sha256sum }}
{{- end }}

{{- define "rift.probes" -}}
{{- $p := .root.Values.probes -}}
startupProbe:
  httpGet:
    path: /readyz
    port: {{ .readyPort }}
  periodSeconds: {{ $p.startup.periodSeconds }}
  timeoutSeconds: {{ $p.startup.timeoutSeconds }}
  failureThreshold: {{ .startupFailureThreshold | default $p.startup.failureThreshold }}
readinessProbe:
  httpGet:
    path: /readyz
    port: {{ .readyPort }}
  periodSeconds: {{ $p.readiness.periodSeconds }}
  timeoutSeconds: {{ $p.readiness.timeoutSeconds }}
  failureThreshold: {{ $p.readiness.failureThreshold }}
livenessProbe:
  httpGet:
    path: /livez
    port: {{ .livePort | default .readyPort }}
  periodSeconds: {{ $p.liveness.periodSeconds }}
  timeoutSeconds: {{ $p.liveness.timeoutSeconds }}
  failureThreshold: {{ $p.liveness.failureThreshold }}
{{- end }}

{{- define "rift.preStop" -}}
lifecycle:
  preStop:
    exec:
      command: ["sleep", {{ .Values.shutdown.preStopSeconds | quote }}]
{{- end }}

{{- define "rift.podScheduling" -}}
{{- with .Values.nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .Values.tolerations }}
tolerations:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .Values.affinity }}
affinity:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .Values.imagePullSecrets }}
imagePullSecrets:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}

{{- define "rift.gatewayConfig" -}}
{{- $realIp := list }}
{{- range .Values.gateway.trustedProxies }}
{{- $realIp = append $realIp (printf "set_real_ip_from %s;" .) }}
{{- end }}
{{- if $realIp }}
{{- $realIp = append $realIp "real_ip_header X-Forwarded-For;" }}
{{- $realIp = append $realIp "real_ip_recursive on;" }}
{{- end }}
{{- .Files.Get "files/nginx.conf.template" | replace "__NGINX_REAL_IP__" (join "\n    " $realIp) -}}
{{- end }}
