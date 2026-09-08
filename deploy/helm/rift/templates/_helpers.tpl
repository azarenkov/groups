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
