# 10. Project management and risks

## Tools and links

| What | Where |
|---|---|
| Repository | https://github.com/azarenkov/groups |
| Task board (GitHub Projects) with priorities, estimates, assignees and start/target dates | [Groups tracker](https://github.com/users/azarenkov/projects/8) |
| Pre-defense tags | [`pre-defense-1`](https://github.com/azarenkov/groups/releases/tag/pre-defense-1); `pre-defense-2` and `final-pre-defense` follow |

## Workflow

- **`main`** is the reviewed baseline. Every change comes through a pull request from a task branch.
- **Branch names** start with the task ID, for example `63-predefense-docs` for issue #63, as in the checklist's `DIP-23-database-schema` example.
- **Commit messages** use a type and scope (`feat(attachments): …`, `fix(auth): …`). The pull request description links the task with `Closes #N`, explains the change and describes how it was checked.
- **Review.** Another team member reviews every pull request before it is merged. The supervisor, Zhaksylyk Kozhakhmet, has access to the repository and the task board.
- **CI** must pass before merging: linting with warnings as errors, unit and integration tests, the SQL check and the frontend build ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- **Task completion.** A task is moved to Done only when its acceptance criteria are checked.

## Responsibilities

| Member | Area | Tasks (issues) |
|---|---|---|
| Azarenkov Alexey ([@azarenkov](https://github.com/azarenkov)) | Team lead; back-end services, data, messaging, real-time media, deployment, documentation | Back-end parts of every feature: #22, #24, #26, #28, #30, #32, #34, #36, #38, #40, #42, #44, #46, #48, #50, #52, #54, #56, #58, #60; documentation #63 |
| Alihan Manat ([@alishahmanov](https://github.com/alishahmanov)) | Front-end: messaging experience | #23 reactions, #25 threads, #31 announcements, #37 polls, #41 meeting controls, #43 presence, #45 rich text, #47 saved messages, #51 recordings, #53 breakout rooms, #59 class notes, #61 translation |
| Iskander Akipov ([@Iskander1612](https://github.com/Iskander1612)) | Front-end: study workflow | #27 assignments, #29 meetings and calendar, #33 push notifications, #35 document viewer, #39 files tab, #49 class code, #55 attendance, #57 whiteboard |

Each feature (issues #2–#21) is split into a back-end and a front-end sub-issue, so every feature has a named owner per layer. Priorities (P0–P2), sizes, estimates in days and start and target dates are set on the task board.

## Milestones

| Milestone | Date | Content | Status |
|---|---|---|---|
| Pre-defense 1 | 20 Oct 2026 | Teams-like layout, student and teacher roles, groups instead of communities; tag `pre-defense-1` | Done |
| Documentation checkpoint | 24 Oct 2026 | Documents 1–11 of this folder, README, diagrams, prototype, task board | This pull request |
| Document viewer | 21 Oct 2026 | In-app viewer for PDF and office files (#21) | Done |
| P0 features complete | 28 Dec 2026 | Reactions, threads, assignments, meetings and calendar, announcements, push notifications | Planned |
| Pre-defense 2 | _date from the department_ | P0 features, first load-test and usability results | Planned |
| P1 features complete | 17 Feb 2027 | Polls, files tab, meeting controls, presence, rich text, saved messages, class code | Planned |
| P2 features complete | 22 Apr 2027 | Recordings and captions, breakout rooms, attendance, whiteboard, class notes, translation | Planned |
| Final pre-defense | _date from the department_ | Complete evaluation report, tag `final-pre-defense` | Planned |

The dates come from the task board, where the tasks of each member are scheduled one after another from their estimates, with weekends and public holidays excluded.

## Dependencies

| Dependency | Type | Affects |
|---|---|---|
| A back-end sub-issue must expose its API before the front-end sub-issue can be completed | Internal, task order | Every feature with two sub-issues |
| Assignments (#4) reuse the attachment pipeline and the document viewer (#21) | Internal | FR-21 |
| Meeting controls, recordings, breakout rooms and attendance (#10, #15–#17) build on LiveKit room APIs | Internal / external | FR-42…FR-44 |
| Push notifications (#7) need browser Web Push and a VAPID key pair | External | FR-51 |
| Email delivery needs an SMTP account | External | FR-01, FR-02 |
| Interviews and the usability study need access to students and teachers | External | Documents 2 and 9 |
| Hosting for the pilot needs a server or cluster at the university | External (client) | NFR-PRIV-1, evaluation |

## Risks

| ID | Risk | Probability | Impact | Mitigation |
|---|---|---|---|---|
| R1 | All back-end work sits with one developer, who is the critical path of the schedule | High | High | P0 features first; the front-end developers start each feature against the agreed API contract; P2 features are dropped first if the schedule slips |
| R2 | Uneven visible contribution between team members at the pre-defense | High | High | Front-end work is committed in small pull requests from the first day of each task; every member reviews pull requests; see [Contribution evidence](#contribution-evidence) |
| R3 | Too few interview participants or a late usability study | Medium | High | Interview guide ready now; recruit from the team's own study groups; online sessions allowed |
| R4 | Call quality on weak university Wi-Fi | Medium | Medium | Audio-first voice rooms, simulcast and bandwidth adaptation; measured explicitly in the plan |
| R5 | Scope creep from the long feature list | Medium | Medium | Fixed priorities; P1 and P2 only after P0 is accepted |
| R6 | No university server for the pilot | Medium | Medium | The pilot runs on a team member's server with Docker Compose; the Helm chart stays ready for the university cluster |
| R7 | Security incident with untrusted files | Low | High | Sandboxed converter without network access, CSP sandbox on previews, malware-like files rejected by type and size checks |
| R8 | Data loss in an upgrade | Low | High | Migrations reviewed in pull requests; database backups before each deployment; outbox makes events durable |

## Contribution evidence

Status on 23 October 2026, from the Git history and the task board:

- **Azarenkov Alexey (@azarenkov):**
  - All commits on `main` so far, including the history of the platform this project started from;
  - pull requests [#1](https://github.com/azarenkov/groups/pull/1) (Teams-like design and roles) and [#62](https://github.com/azarenkov/groups/pull/62) (document viewer);
  - this documentation.
- **Alihan Manat (@alishahmanov):** task #23 (reactions, front-end) is in progress on the task board. No commits on `main` yet.
- **Iskander Akipov (@Iskander1612):** tasks assigned (#27, #29, #33, #35, …). No commits on `main` yet.

Planned actions before the next checkpoint:

- Each front-end member opens a pull request for their first task.
- Each member reviews at least one pull request of another member. This documentation pull request is the first one.

Contributions are then visible in the commit history, in pull request reviews and on the task board.
