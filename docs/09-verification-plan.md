# 9. Verification and evaluation plan

The aim — one self-hostable system that covers group communication, meetings and the study workflow ([`01-project-proposal.md`](01-project-proposal.md#aim)) — is demonstrated on three levels:

1. **Correctness.** Every mandatory requirement passes its acceptance criteria (test matrix below).
2. **Quality attributes.** Performance, reliability and security targets are measured under defined conditions.
3. **Value for users.** A usability and acceptance study compares Groups with the tools groups use today (the baseline).

## Test levels

| Level | What | Tools | Where it runs |
|---|---|---|---|
| Unit | Domain rules, use cases with fake ports, adapters' mapping code | `cargo test`, Jest (React Testing Library) | CI on every pull request |
| Integration | Repositories against PostgreSQL, outbox ordering and dispatch, event contracts round-trip | `cargo test` with `RIFT_TEST_DATABASE_URL`, sqlx offline cache checks | CI (`outbox_ordering` job) |
| Static checks | Lints with warnings as errors, SQL built only from literals, secret scan | `cargo clippy -D warnings`, `scripts/check_sql.py`, secret-scan workflow | CI |
| End-to-end | Key scenarios in a real browser against a full deployment | Playwright (planned), manual test cases | Local Docker Compose or kind cluster |
| Load | HTTP, WebSocket and call load | k6 (HTTP and WebSocket), LiveKit CLI `lk load-test` | Reference environment |
| Fault injection | Restart services and brokers during traffic | `kubectl delete pod`, `docker compose restart` | Reference environment |
| Usability | Task-based sessions and questionnaires | SUS, TAM items, ZEF Scale | With students and teachers |

The CI configuration is in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml). A pull request cannot be merged while CI is red.

## Test matrix

| Requirement | Verification | Type | Status |
|---|---|---|---|
| FR-01, FR-02 | Registration, verification, login, reset tests in `identity` core and API; manual case TC-01 | Unit, manual | Automated tests exist |
| FR-03, FR-10 | Group and channel use-case tests in `messaging` core; role check returns 403 for students | Unit | Automated tests exist |
| FR-11, FR-12 | Message use cases, outbox mapping tests, realtime dispatch tests | Unit, integration | Automated tests exist |
| FR-14, FR-15 | New use-case tests and an end-to-end case per issue | Unit, E2E | Planned with #2, #3 |
| FR-20, FR-21, FR-22 | Use-case tests per acceptance criterion; E2E cases TC-05…TC-07 | Unit, E2E | Planned with #4, #5, #6 |
| FR-30 | Preview use-case tests (conversion once, cache, access rules), converter adapter test against a live Gotenberg, viewer component tests | Unit, integration | Automated tests exist |
| FR-40, FR-41 | Voice use cases (join, mute, deafen, enforcement budget), LiveKit adapter tests; manual call test TC-04 | Unit, manual | Automated tests exist |
| FR-50, FR-51 | Inbox consumer tests; push delivery E2E | Unit, E2E | Inbox done; push planned with #7 |
| NFR-PERF-1, NFR-PERF-2 | k6 scenario: 200 virtual users, each connected over WebSocket, sending one message per 10 s and reading channels | Load | Planned |
| NFR-PERF-3 | `lk load-test` with 30 participants, 1 screen share and 4 cameras; join time measured in the browser | Load | Planned |
| NFR-REL-1, NFR-REL-2 | Ordering and de-duplication tests on PostgreSQL (exist); fault-injection run: restart each service during a 1 000-message burst and compare received sequences | Integration, fault injection | Ordering tests exist; fault run planned |
| NFR-SEC-1…4 | Revocation tests; converter runs with no egress (NetworkPolicy); CSP header test; CI secret scan; rate-limit tests | Unit, configuration review | Partly automated |
| NFR-USE-1, NFR-USE-2 | Usability study below | Study | Planned |
| NFR-COMP-1 | Manual run of TC-01…TC-07 on Chrome, Firefox, Safari, Edge, the desktop app on Windows, macOS and Linux, and a 360 px phone viewport | Manual | Planned |
| NFR-OPS-1 | `docker compose up` on a clean machine; Helm install on a kind cluster with all probes green | Manual, scripted | Done for the current version |

### Manual test cases for the key scenarios

| ID | Scenario | Steps | Expected result |
|---|---|---|---|
| TC-01 | Sign up as a student | Open sign-up, choose Student, fill the form, confirm the code | Account active, role Student, lands in the app |
| TC-02 | Teacher creates a group and invites a student | Teacher creates a group and a channel, sends an invitation link; student opens it | Student is a member and sees the channel |
| TC-03 | Read a shared document | Upload a DOCX and a PDF to a channel; open both | Both open in the viewer with pages; download works |
| TC-04 | Online lesson | Teacher starts a call in a voice channel and shares the screen; 3 students join, one with camera off; teacher mutes one student | All see the shared screen; mute takes effect within 5 s |
| TC-05 | Assignment cycle | Teacher creates an assignment; student turns in a file; teacher grades and returns | Student sees grade and feedback; late flag when after due date |
| TC-06 | Announcement | Teacher posts in an announcement-only channel; student tries to post | Every member is notified; student cannot post |
| TC-07 | Meeting from the calendar | Teacher schedules a meeting; student opens the calendar and joins | Reminder arrives; joining opens the call |

## Metrics

| Metric | Target | Source requirement |
|---|---|---|
| Message delivery latency (send request → recipient's WebSocket), p95 | ≤ 500 ms at 200 concurrent users | NFR-PERF-1 |
| API latency p95, read / write | ≤ 300 ms / ≤ 500 ms | NFR-PERF-2 |
| Call join time p95 | ≤ 3 s | NFR-PERF-3 |
| One-way media latency in a 30-person call | ≤ 400 ms | NFR-PERF-3 |
| Messages lost or reordered in the fault-injection run | 0 | NFR-REL-1 |
| Duplicates after redelivery | 0 | NFR-REL-2 |
| Throughput gain from 1 to 2 replicas | ×2 ± 20 % | NFR-SCAL-1 |
| System Usability Scale | ≥ 70 | NFR-USE-1 |
| Task time: student joins a group and sends a message / teacher posts an assignment | ≤ 2 min / ≤ 3 min | NFR-USE-2 |
| Task success rate in the usability study | ≥ 90 % | NFR-USE-2 |
| Requirements with passing acceptance tests | 100 % of P0 | All P0 FR |

## Measurement conditions

- **Reference environment.** A local kind cluster with three nodes (Docker Desktop or OrbStack on a developer laptop), or a single server with Docker Compose. Hardware, CPU count, memory and software versions are recorded with every result.
- **Network.** Load tests run from the same LAN. The call test is repeated on a wireless connection limited to 20 Mbit/s with 50 ms added latency (`tc netem` or the browser's network throttling), following the finding that WebRTC behaves worse on wireless links ([S14]).
- **Warm-up.** Results are taken after a 2-minute warm-up. Each run lasts at least 10 minutes and is repeated three times. The median of the three p95 values is reported.
- **Data.** 3 groups, 10 channels, 200 users, 50 000 seeded messages, so that queries do not run on empty tables.
- **Versions.** Every result references the Git tag or commit that was measured.

## Usability study

- **Participants:** at least 8 students and 2 teachers, recruited from the interview participants and their groups.
- **Procedure:**
  1. A short introduction.
  2. Tasks TC-01 … TC-07 performed in Groups and in the baseline toolset. The order is counterbalanced: half of the participants start with each.
  3. Questionnaires.
  4. A short interview.
- **Instruments:**
  - System Usability Scale for both systems;
  - perceived usefulness and ease-of-use items from the Technology Acceptance Model ([S9]);
  - Zoom Exhaustion & Fatigue Scale after the call task ([S7]).
- **Measures:** task success, task time, number of tool switches needed to complete a task, and questionnaire scores.

## Baseline for comparison

The baseline is **the toolset study groups use today**, as recorded in the interviews ([`02-relevance-evidence.md`](02-relevance-evidence.md)). It is expected to be a messenger for chat, a video service for calls and a learning management system for assignments. If a group already uses Microsoft Teams for Education, Teams is the baseline for that group. The comparison uses the same tasks and measures. Groups is considered successful if:

- it reaches the SUS target;
- tasks need fewer tool switches than in the baseline;
- task times are not worse than in the baseline.

For quality attributes the baseline is the target values above, because the internal performance of proprietary systems cannot be measured on our infrastructure.

## Reporting

Results go to `docs/evaluation/` with the raw data (k6 summaries, LiveKit load-test output, questionnaire spreadsheets with anonymised participants) and a short report per pre-defense.
