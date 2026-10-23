# 6. Requirements specification

**Priorities:** **P0** — must have for the diploma, **P1** — should have, **P2** — could have. The same priorities are set on the tasks in the [task board](https://github.com/users/azarenkov/projects/8).
**Status:** _Implemented_ — available in the current build and covered by automated tests; _Planned_ — scheduled task.
**Hypotheses** H1–H7 come from [`02-relevance-evidence.md`](02-relevance-evidence.md).

Each acceptance criterion is written so that it can be checked by an automated test, a manual test case in [`09-verification-plan.md`](09-verification-plan.md), or a measurement.

## Functional requirements

### Accounts and roles

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-01 | A user registers with a name, email and password and chooses the role **student** or **teacher**. | P0 | Registration is rejected for a taken name or email, a weak password or a failed captcha. The account is active only after the emailed code is confirmed. The chosen role is stored and returned in the session. | Implemented | — |
| FR-02 | A user signs in, signs out and resets a forgotten password by email. | P0 | Wrong credentials are rejected and repeated failures are rate-limited. A reset code expires and works once. Signing out invalidates the session. A password change or a disabled account revokes all sessions of the user. | Implemented | — |
| FR-03 | Only teachers create groups; students join by invitation. | P0 | A student's request to create a group is refused with 403. A teacher's request succeeds and the teacher becomes the owner. | Implemented | — |

### Groups, channels and messages

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-10 | A group has text and voice channels, roles with permissions and invitation links. | P0 | The owner creates, renames and deletes channels and assigns roles. An invitation link adds a member. Removed members lose access immediately. | Implemented | — |
| FR-11 | Members send, edit and delete messages with mentions, attachments, stickers, voice messages and video notes. | P0 | A sent message reaches all online members without a page reload. Editing and deleting are visible to all members. Mentioned users receive a notification. Content longer than 2000 characters is rejected. | Implemented | — |
| FR-12 | Direct messages between two users, with read state. | P0 | Only the two participants see the conversation. The sender sees when the message was read. A blocked user cannot send a message. | Implemented | — |
| FR-13 | Search messages and see pinned messages. | P1 | Search returns only messages the user is allowed to read. Pins are visible to all members of the conversation. | Implemented | — |
| FR-14 | Reactions on channel and direct messages. (H1) | P0 | A member adds and removes an emoji reaction. All members see the counts update in real time. A user cannot add the same reaction twice. | Planned | [#2](https://github.com/azarenkov/groups/issues/2) |
| FR-15 | Threaded replies in group channels. (H1, H2) | P0 | A reply is attached to its parent message. The parent shows the number of replies. Thread participants are notified about new replies. | Planned | [#3](https://github.com/azarenkov/groups/issues/3) |
| FR-16 | Rich text formatting: bold, italic, code, lists, links. | P1 | The formatting is preserved after sending, editing and in notifications. Unsafe HTML is not rendered. | Planned | [#12](https://github.com/azarenkov/groups/issues/12) |
| FR-17 | Saved messages and scheduled sending. | P1 | A scheduled message is delivered within one minute of its time. Saved messages are visible only to their owner. | Planned | [#13](https://github.com/azarenkov/groups/issues/13) |
| FR-18 | Polls and quizzes in chat. | P1 | A member votes once per poll. Results update in real time. A quiz shows the correct answer after it is closed. | Planned | [#8](https://github.com/azarenkov/groups/issues/8) |

### Study workflow

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-20 | Announcements and teacher-only posting in channels. (H2) | P0 | A teacher marks a channel as announcement-only. Students can read but not post there. An announcement notifies every member. | Planned | [#6](https://github.com/azarenkov/groups/issues/6) |
| FR-21 | Assignments: a teacher creates an assignment with instructions, files, due date and points; students submit; the teacher grades and returns feedback. (H4) | P0 | A student attaches files and turns in, and can undo until the work is returned. Late submissions are flagged. A teacher grades and returns, and the student is notified. Students never see other students' submissions or grades. | Planned | [#4](https://github.com/azarenkov/groups/issues/4) |
| FR-22 | Scheduled meetings and a group calendar. (H2, H5) | P0 | A teacher schedules a meeting with a date, time and duration. Members see it in the calendar and get a reminder. Joining from the event opens the call. | Planned | [#5](https://github.com/azarenkov/groups/issues/5) |
| FR-23 | Join a group by class code and bulk import of members. | P1 | A valid code adds the student to the group. An expired code is rejected. A CSV import reports each row that failed. | Planned | [#14](https://github.com/azarenkov/groups/issues/14) |
| FR-24 | A files tab with folders per group. | P1 | Members browse all files shared in the group. Teachers create folders. Access follows group membership. | Planned | [#9](https://github.com/azarenkov/groups/issues/9) |
| FR-25 | Class notes or a wiki tab per group. | P2 | Members with permission edit pages, and the edit history is kept. | Planned | [#19](https://github.com/azarenkov/groups/issues/19) |

### Documents

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-30 | View PDF, Word, PowerPoint, Excel, CSV, JSON and text attachments inside the app without downloading. (H3) | P0 | The listed formats open in the viewer, with pages, zoom and download. Office files are converted on the server once and the result is cached. Access follows the attachment's visibility rules. A file that cannot be converted shows an error and a download link. | Implemented | [#21](https://github.com/azarenkov/groups/issues/21) |

### Calls and meetings

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-40 | Voice and video calls in voice channels and direct messages, with screen sharing. (H5) | P0 | Members join a voice channel and see who is connected. Camera and screen sharing can be switched on and off. A direct call rings the other user. Disconnects are recovered automatically. | Implemented | — |
| FR-41 | A teacher mutes, deafens or removes a participant. | P0 | The action takes effect for the participant's media within 5 seconds and is visible to all participants. | Implemented | — |
| FR-42 | Meeting controls: raise hand, in-call reactions, host actions. | P1 | Raised hands are listed in order. The host can lower hands and mute everyone. | Planned | [#10](https://github.com/azarenkov/groups/issues/10) |
| FR-43 | Recordings, live captions and transcripts. | P2 | A recording is stored in the group's files. Captions appear within 3 seconds of speech. | Planned | [#15](https://github.com/azarenkov/groups/issues/15) |
| FR-44 | Breakout rooms and attendance reports. | P2 | The host splits participants into rooms and brings them back. The attendance report lists join and leave times. | Planned | [#16](https://github.com/azarenkov/groups/issues/16), [#17](https://github.com/azarenkov/groups/issues/17) |
| FR-45 | Collaborative whiteboard. | P2 | Participants draw on a shared board and see each other's strokes in real time. | Planned | [#18](https://github.com/azarenkov/groups/issues/18) |

### Notifications and presence

| ID | Requirement | Priority | Acceptance criteria | Status | Task |
|---|---|---|---|---|---|
| FR-50 | An in-app inbox of notifications (mentions, invitations, friend requests). | P0 | New items appear without a reload and can be marked as read. | Implemented | — |
| FR-51 | Browser and desktop push notifications. (H2) | P0 | A user who allowed notifications receives a push for mentions, announcements and assignment events while the app is closed. Notification settings per group are respected. | Planned | [#7](https://github.com/azarenkov/groups/issues/7) |
| FR-52 | Presence states: available, busy, away, do not disturb. | P1 | The chosen state is visible to group members. Do not disturb suppresses push notifications. | Planned | [#11](https://github.com/azarenkov/groups/issues/11) |
| FR-53 | Message translation. | P2 | A member translates a message into the interface language on request. | Planned | [#20](https://github.com/azarenkov/groups/issues/20) |

## Non-functional requirements

| ID | Category | Requirement | Measure and acceptance criterion |
|---|---|---|---|
| NFR-PERF-1 | Performance | Messages reach online recipients quickly. | p95 of the time from a send request to the event arriving at a recipient's WebSocket ≤ **500 ms**, with 200 concurrent connected users on the reference environment ([`09`](09-verification-plan.md#measurement-conditions)). |
| NFR-PERF-2 | Performance | API calls are responsive. | p95 latency of read endpoints ≤ **300 ms**, and of write endpoints ≤ **500 ms**, under the same load. |
| NFR-PERF-3 | Performance | Calls are usable for a lesson. | Joining a call takes ≤ **3 s** (p95). A room with **30** participants, one screen share and 4 cameras keeps one-way media latency ≤ **400 ms** on a 20 Mbit/s connection. |
| NFR-REL-1 | Reliability | No message or event is lost or reordered within a conversation when a service restarts. | Fault-injection test: restart each service during a send burst of 1 000 messages. All messages arrive exactly once per recipient and in send order. |
| NFR-REL-2 | Reliability | Consumers are idempotent. | Redelivering any event does not create duplicates (automated tests). |
| NFR-SEC-1 | Security | Traffic is encrypted, and sessions can be revoked. | TLS on all public endpoints. Access tokens are short-lived. Signing out or disabling an account revokes the session within one token lifetime. |
| NFR-SEC-2 | Security | Untrusted files are isolated. | The document converter runs with no network egress, a read-only file system and no Linux capabilities. Previews are served with `Content-Security-Policy: sandbox`. |
| NFR-SEC-3 | Security | Secrets are not stored in the repository. | The CI secret scan finds nothing. Configuration uses example files with placeholders. |
| NFR-SEC-4 | Security | Abuse is limited. | Login, registration and message sending are rate-limited. Images and text pass automatic moderation. |
| NFR-PRIV-1 | Privacy | All user data can stay on the university's own servers. | Every component, including media, storage and conversion, is self-hosted. No third-party cloud service is required. |
| NFR-PRIV-2 | Privacy | A user can delete their account. | Deletion removes the profile and the user's files within the documented retention time. |
| NFR-SCAL-1 | Scalability | HTTP services scale horizontally. | Running two replicas of each stateless service doubles throughput within ±20 % in the load test. |
| NFR-AVAIL-1 | Availability | A rolling update does not interrupt users. | During a Helm upgrade, connected clients reconnect automatically and no request fails with a 5xx for more than 10 s. |
| NFR-USE-1 | Usability | The application is easy to learn. | System Usability Scale score ≥ **70** with at least 8 participants ([`09`](09-verification-plan.md#usability-study)). |
| NFR-USE-2 | Usability | Key tasks are fast. | A first-time student joins a group and sends a message in ≤ 2 min. A teacher posts an assignment in ≤ 3 min. |
| NFR-COMP-1 | Compatibility | The app runs on common platforms. | Current Chrome, Firefox, Edge and Safari. The desktop app runs on Windows, macOS and Linux. The layout works from 360 px width. |
| NFR-I18N-1 | Localisation | The interface is translated. | English and Russian, with no untranslated strings in the key scenarios. |
| NFR-MAINT-1 | Maintainability | The code is checked automatically. | CI runs linting with warnings as errors, unit and integration tests, and a check that SQL is never built by string formatting. A pull request cannot merge on a red build. |
| NFR-OPS-1 | Operability | The system is deployable. | `docker compose up` starts the full system on one server. The Helm chart deploys it to Kubernetes, with health and readiness probes on every service. |

## Traceability

- **Requirements → tasks:** the Task column above, and the [task board](https://github.com/users/azarenkov/projects/8).
- **Requirements → tests:** the test matrix in [`09-verification-plan.md`](09-verification-plan.md#test-matrix).
- **Requirements → components:** the component diagram in [`07-architecture.md`](07-architecture.md#components).
