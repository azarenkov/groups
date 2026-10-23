# Groups

**A self-hosted collaboration platform for study groups.** Group channels and direct messages, voice and video calls with screen sharing, an in-app viewer for PDF and Office documents, and a study workflow (announcements, assignments, scheduled meetings) in one system that a university runs on its own servers.

- **Task board:** [Groups tracker](https://github.com/users/azarenkov/projects/8) — priorities, estimates, assignees, start and target dates
- **YouTrack:** _link to be added by the team_
- **Documentation:** [`docs/`](docs) — proposal, research, requirements, architecture, prototype, verification plan

## Problem, users and expected outcomes

Study groups split their work across a messenger, a video service, a learning management system and cloud storage. The integrated alternatives — Microsoft Teams for Education and Google Classroom — cannot be hosted by the university and keep data region control behind paid editions ([comparison](docs/04-existing-systems.md)).

| Users | What they get |
|---|---|
| Students | One place for group chat, calls, files, deadlines and grades, on the web, desktop and phone |
| Teachers | Groups per class, announcements, online lessons with screen sharing, assignments and grading |
| University IT | A system deployable on its own servers with Docker Compose or Kubernetes |

**Expected outcomes:**

- a web app, a desktop app and a responsive mobile web view;
- tested back-end services;
- documented deployment;
- an evaluation against measurable targets ([verification plan](docs/09-verification-plan.md)).

## Team

| Member | Responsibilities |
|---|---|
| [@azarenkov](https://github.com/azarenkov) — _full name_ | Team lead; back-end services, data, messaging, real-time media, deployment, documentation |
| [@alishahmanov](https://github.com/alishahmanov) — _full name_ | Front-end: chat, threads, reactions, announcements, presence, rich text, meeting controls |
| [@Iskander1612](https://github.com/Iskander1612) — _full name_ | Front-end: assignments, calendar and meetings, push notifications, files, document viewer |

**Supervisor:** _name and position_

Responsibilities per task are in [`docs/10-project-management.md`](docs/10-project-management.md#responsibilities).

## Scope and current status

| Area | Status |
|---|---|
| Accounts with student and teacher roles, email verification, password reset | Implemented |
| Groups (teacher-created) with text and voice channels, roles, invitations | Implemented |
| Channel and direct messages: edit, delete, mentions, pins, attachments, stickers, voice messages, video notes, read state, search | Implemented |
| In-app viewer for PDF, Word, PowerPoint, Excel, CSV, JSON and text | Implemented ([#21](https://github.com/azarenkov/groups/issues/21)) |
| Voice and video calls, screen sharing, teacher moderation (mute, deafen, remove) | Implemented |
| Notification inbox | Implemented |
| Desktop app (Windows, macOS, Linux) | Implemented |
| Reactions, threads, announcements, assignments, meetings and calendar, push notifications | Planned, P0 — by 28 Dec 2026 |
| Polls, files tab, meeting controls, presence states, rich text, saved messages, class code | Planned, P1 — by 17 Feb 2027 |
| Recordings and captions, breakout rooms, attendance, whiteboard, class notes, translation | Planned, P2 — by 22 Apr 2027 |

The full requirement list with acceptance criteria is in [`docs/06-requirements.md`](docs/06-requirements.md). Pre-defense versions are tagged: [`pre-defense-1`](https://github.com/azarenkov/groups/releases/tag/pre-defense-1).

**Known limitations:**

- Parts of the interface and the service names still use the name of the platform the project started from ("Rift").
- There are no native mobile apps; the mobile experience is the responsive web app.

## Technologies

| Layer | Choice |
|---|---|
| Back-end | Rust — actix-web, tokio, sqlx; six services and two workers |
| Data | PostgreSQL (one database per service), Redis, MinIO (S3) |
| Events | NATS JetStream with a transactional outbox |
| Real-time | WebSocket service; LiveKit SFU for WebRTC calls |
| Documents | Gotenberg (LibreOffice) in a sandbox, pdf.js in the client |
| Front-end | React, TypeScript, React Query, Zustand, i18next; Electron for the desktop |
| Deployment | Docker Compose; Kubernetes with Helm and Terraform; GitHub Actions CI |

Alternatives and reasons: [`docs/05-technology-comparison.md`](docs/05-technology-comparison.md).

## Documentation

| # | Document |
|---|---|
| 1 | [Project proposal and scope](docs/01-project-proposal.md) |
| 2 | [Evidence of relevance](docs/02-relevance-evidence.md) (interview guide and protocols) |
| 3 | [Literature review](docs/03-literature-review.md) (21 sources, 17 peer-reviewed) |
| 4 | [Existing system comparison](docs/04-existing-systems.md) |
| 5 | [Technology comparison](docs/05-technology-comparison.md) |
| 6 | [Requirements specification](docs/06-requirements.md) |
| 7 | [Architecture and diagrams](docs/07-architecture.md) — sources in [`docs/diagrams`](docs/diagrams) |
| 8 | [UX and prototype](docs/08-ux-prototype.md) — mockups in [`docs/prototype`](docs/prototype) |
| 9 | [Verification and evaluation plan](docs/09-verification-plan.md) |
| 10 | [Project management and risks](docs/10-project-management.md) |
| 11 | [Supervisor feedback](docs/11-supervisor-feedback.md) |

## Repository structure

```
backend/
  services/<name>/{domain,core,infrastructure,api,migrations}   identity, messaging, media, voice, realtime, notification
  workers/                                                      outbox-worker, livekit-pooler
  libs/                                                         shared kernel, event contracts, ports, infrastructure helpers
frontend/                                                       React app and Electron desktop shell
docker/                                                         Dockerfiles per service
deploy/                                                         Helm chart, Terraform environments, Argo CD values
docs/                                                           project documentation, diagrams, prototype
scripts/                                                        SQL check, database helpers
.env.example                                                    configuration with placeholder values
```

## Running

### Everything with Docker Compose

Requirements: Docker with Compose v2.

```bash
mkdir -p env
for f in http identity-service media-service messaging-service voice-service realtime notification outbox livekit livekit-pooler nats postgres websocket; do
  cp .env.example env/$f.env
done
# edit the secrets in env/*.env (JWT_SECRET, ALTCHA_HMAC_KEY, INTERNAL_API_TOKEN, MAILER_*)
docker compose up -d --build
```

The app is served at http://localhost:8080. Without `MAILER_*` settings, verification emails are not sent.

### Kubernetes on a local kind cluster

```bash
make -C deploy local-up
make -C deploy trust-ca
```

The app is at https://rift.localtest.me. Details: [`deploy/README.md`](deploy/README.md).

### Front-end development server

```bash
cd frontend
npm ci
npm start
```

## Testing

```bash
SQLX_OFFLINE=true cargo test --workspace
SQLX_OFFLINE=true cargo clippy --workspace -- -D warnings
python3 scripts/check_sql.py
cd frontend && npm run test:ci && npm run build
```

Database integration tests, for example outbox ordering, need a PostgreSQL admin URL:

```bash
RIFT_TEST_DATABASE_URL=postgres://postgres:<password>@localhost:5432/postgres \
  SQLX_OFFLINE=true cargo test -p infrastructure-common --test outbox_ordering -- --include-ignored
```

The test plan, metrics and measurement conditions are in [`docs/09-verification-plan.md`](docs/09-verification-plan.md).

## Contributing

Work happens on task branches named after the issue (for example `63-predefense-docs`). Changes go through pull requests that reference the issue and are reviewed by another team member. See [`docs/10-project-management.md`](docs/10-project-management.md#workflow).
