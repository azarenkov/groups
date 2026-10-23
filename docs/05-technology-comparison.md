# 5. Technology comparison

Each key decision is compared against criteria derived from the requirements in [`06-requirements.md`](06-requirements.md). Scores: **++** strong fit, **+** fit, **0** neutral, **−** weak fit. The chosen option is in **bold**. Licence and feature facts link to the official project pages listed in [References](#references).

Requirement drivers used as criteria:

- **NFR-PERF** — real-time delivery and calls with low latency for groups of 20–40 people.
- **NFR-REL** — no lost or reordered messages when a service restarts.
- **NFR-SEC** — memory-safe handling of untrusted input (uploads, documents, WebSocket frames).
- **NFR-HOST** — self-hostable with open-source components that a university may run on its own servers.
- **NFR-OPS** — deployable on one server and on a Kubernetes cluster.
- **TEAM** — what the team already knows and can maintain within the project time.

## D1. Back-end platform

| Criterion | **Rust (actix-web, tokio)** | Go (net/http, gin) | Node.js / TypeScript (NestJS) |
|---|---|---|---|
| Throughput and latency under concurrent WebSocket and HTTP load (NFR-PERF) | ++ async runtime without GC pauses | ++ goroutines, GC pauses are short | + single-threaded event loop per process |
| Memory safety for untrusted input (NFR-SEC) | ++ enforced by the compiler | + memory-safe with GC, data races possible | + memory-safe, dynamic typing at runtime |
| Compile-time checked SQL (NFR-REL) | ++ sqlx checks queries against the schema at build time | + sqlc generates code from SQL | 0 mostly runtime ORMs |
| Ecosystem for LiveKit, NATS, S3, Postgres | + official or maintained crates for all | ++ | ++ |
| Team experience (TEAM) | + backend developer works in Rust | 0 | + frontend developers know TypeScript |
| Build time and learning curve | − long compile times, steep curve | ++ | ++ |

**Decision: Rust.** Memory safety and compile-time checked SQL directly serve NFR-SEC and NFR-REL. Long build times are mitigated with per-service CI jobs and cached dependency layers in Docker builds.

## D2. Primary data storage

| Criterion | **PostgreSQL** | MongoDB | MySQL |
|---|---|---|---|
| Transactions across several tables, needed for the outbox pattern (NFR-REL) | ++ | + multi-document transactions on replica sets | ++ |
| Relational data: groups, members, roles, messages, read state | ++ | 0 documents need manual references | ++ |
| JSONB for event payloads, arrays, partial indexes, advisory locks | ++ | ++ native documents | + JSON type, no advisory locks per key |
| Licence suitable for self-hosting (NFR-HOST) | ++ PostgreSQL License (permissive) | − SSPL | + GPLv2 (community edition) |
| Compile-time checked queries with sqlx | ++ | − not supported by sqlx | + |

**Decision: PostgreSQL**, one database per service so that services do not share tables.

**Object storage for files:** **MinIO (S3 API)** vs local file system vs a cloud S3 service. MinIO keeps the S3 API, so a university can switch to any S3-compatible store, while still running on its own servers (NFR-HOST). The local file system does not scale beyond one node; a public cloud store conflicts with data residency (H6 in [`02-relevance-evidence.md`](02-relevance-evidence.md)).

## D3. Interaction between services

| Criterion | **NATS JetStream + transactional outbox** | Apache Kafka | RabbitMQ |
|---|---|---|---|
| Durable streams with replay for projections (NFR-REL) | ++ streams with per-consumer acknowledgement and replay | ++ partitioned log, replay | + quorum queues and streams |
| Operational weight on one server (NFR-OPS) | ++ single small binary | − broker cluster, more memory | + |
| Subject-based routing with wildcards | ++ | 0 topic per stream | ++ topic exchanges |
| Message de-duplication on publish | ++ `Nats-Msg-Id` header | + idempotent producer | 0 |
| Licence (NFR-HOST) | ++ Apache-2.0 | ++ Apache-2.0 | + MPL-2.0 |

**Decision: NATS JetStream for events, plain HTTP for synchronous internal queries.** Each service writes its events into an outbox table in the same database transaction as the state change; a dispatcher publishes them to JetStream. This avoids lost events when a service crashes between writing and publishing (NFR-REL). Synchronous HTTP is used only where a service needs an immediate answer from another, for example claiming an uploaded attachment.

## D4. Real-time media (calls and screen sharing)

| Criterion | **LiveKit (SFU)** | Jitsi Meet | Peer-to-peer mesh WebRTC |
|---|---|---|---|
| Scales to 20–40 participants per room (NFR-PERF) | ++ selective forwarding, simulcast | ++ SFU (Jitsi Videobridge) | − each client uploads one stream per peer |
| Server-side API to create rooms, grant permissions, mute and remove participants | ++ server SDKs and webhooks | + | − no central control |
| Embedding in our own UI | ++ client SDK for the browser, used inside our React app | + mostly as a full meeting application or iframe | + |
| Licence (NFR-HOST) | ++ Apache-2.0 | ++ Apache-2.0 | ++ browser standard |

**Decision: LiveKit.** The server API lets the messaging service enforce teacher moderation (mute, remove, deafen) from the domain rules, which a P2P mesh cannot do and an embedded Jitsi does not expose as directly.

## D5. Real-time delivery to clients

| Criterion | **WebSocket** | Server-Sent Events | Long polling |
|---|---|---|---|
| Bidirectional (typing, presence, acknowledgements) | ++ | − server to client only | − |
| Latency (NFR-PERF) | ++ | ++ | − one request per update |
| Proxy and load-balancer support | + needs upgrade support in the gateway | ++ plain HTTP | ++ |

**Decision: WebSocket** through a dedicated `realtime` service that consumes JetStream events and fans them out to connected users, with per-conversation sequence numbers so that clients detect gaps.

## D6. Client applications

| Criterion | **React + TypeScript** | Vue 3 | Angular |
|---|---|---|---|
| Team experience (TEAM) | ++ | + | 0 |
| LiveKit client components | ++ official React components | + core SDK only | + core SDK only |
| Ecosystem (PDF viewer, i18n, query caching) | ++ | + | + |

| Criterion | **Electron** | Tauri |
|---|---|---|
| Reuses the web build unchanged | ++ | + uses the system web view, rendering differences per OS |
| Screen capture and media APIs consistent with Chrome | ++ bundles Chromium | − depends on the platform web view |
| Bundle size | − large | ++ small |

**Decision: React with TypeScript for the web app, Electron for the desktop app.** The desktop app needs the same WebRTC and screen-sharing behaviour as Chrome, which Electron provides by bundling Chromium.

## D7. Document preview

| Criterion | **Gotenberg (LibreOffice) + pdf.js** | ONLYOFFICE / Collabora document server | Client-side only (pdf.js, SheetJS, docx parsers) |
|---|---|---|---|
| Formats: PDF, DOC(X), PPT(X), XLS(X), ODF | ++ converts office formats to PDF on the server | ++ | − partial, layout differs per format |
| Isolation of untrusted files (NFR-SEC) | ++ separate container, no network egress, read-only file system | + separate server | − parsing runs in the user's browser |
| Resource cost and licence (NFR-HOST) | ++ MIT, one container | − heavy, AGPL or commercial editions | ++ |
| Editing | − view only | ++ collaborative editing | − |

**Decision: Gotenberg + pdf.js.** The requirement is viewing, not editing; a sandboxed converter is the smallest secure option.

## D8. Deployment

| Criterion | **Docker Compose + Kubernetes (Helm, Terraform)** | Virtual machines with systemd units | Managed PaaS |
|---|---|---|---|
| One-server installation for a department (NFR-OPS) | ++ Compose | + | − |
| Cluster deployment with autoscaling and network policies (NFR-OPS, NFR-SEC) | ++ Helm chart, HPA, NetworkPolicy | − manual | + |
| Runs on university hardware (NFR-HOST) | ++ | ++ | − |

**Decision: both Compose and Kubernetes**, built from the same images.

## References

Official project pages used for licence and feature facts (accessed 2026-10-23):

- Rust — https://www.rust-lang.org · actix-web — https://actix.rs · sqlx — https://github.com/launchbadge/sqlx
- Go — https://go.dev · NestJS — https://nestjs.com
- PostgreSQL licence — https://www.postgresql.org/about/licence/ · MongoDB SSPL — https://www.mongodb.com/legal/licensing/server-side-public-license · MySQL — https://www.mysql.com/about/legal/licensing/oem/
- MinIO — https://min.io
- NATS JetStream — https://docs.nats.io/nats-concepts/jetstream · Apache Kafka — https://kafka.apache.org · RabbitMQ — https://www.rabbitmq.com
- LiveKit — https://docs.livekit.io · Jitsi — https://jitsi.org · WebRTC (W3C) — https://www.w3.org/TR/webrtc/
- WebSocket protocol (IETF RFC 6455) — https://datatracker.ietf.org/doc/html/rfc6455 · Server-Sent Events (WHATWG HTML) — https://html.spec.whatwg.org/multipage/server-sent-events.html
- React — https://react.dev · Vue — https://vuejs.org · Angular — https://angular.dev · Electron — https://www.electronjs.org · Tauri — https://tauri.app
- Gotenberg — https://gotenberg.dev · pdf.js — https://mozilla.github.io/pdf.js/ · ONLYOFFICE Docs — https://www.onlyoffice.com/docs · Collabora Online — https://www.collaboraonline.com
- Kubernetes — https://kubernetes.io · Helm — https://helm.sh · Terraform — https://developer.hashicorp.com/terraform
