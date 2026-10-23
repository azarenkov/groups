# 2. Evidence of relevance

This document records **what evidence informs the requirements** and keeps **verified findings separate from assumptions**.

> **Status:** the interview programme below is planned and the protocols are ready. Findings marked _Assumption_ are not yet verified by users. Each interview is recorded with the template in [`evidence/interview-template.md`](evidence/interview-template.md) and stored as `evidence/interview-NN.md`. Personal data is anonymised: participants are referred to by role and number only.

## Method

| Source | What we do | Target | Status |
|---|---|---|---|
| Semi-structured interviews with students | 30-minute conversations following the guide below | 2–3 students from different years | _Planned_ |
| Semi-structured interviews with teachers | Same guide, teacher block | 1–2 teachers who run online or blended courses | _Planned_ |
| Discussion with the client (university IT / supervisor) | Hosting, data storage, identity, operating constraints | 1 meeting | _Planned_ |
| Observation | Watch one group work session (chat + call + file exchange) in the tools the group already uses | 1 session | _Planned_ |
| Literature | Published studies on chat tools, video meetings and LMS use in higher education | 12+ sources | Done — [`03-literature-review.md`](03-literature-review.md) |
| Product documentation | Documented capabilities and limits of existing systems | 5 systems | Done — [`04-existing-systems.md`](04-existing-systems.md) |

Minimum required by the checklist: 3–5 discussions with user representatives, or a substantive agreement with the actual client.

## Interview guide

**Introduction (2 min).** Purpose of the study, anonymity, consent to take notes.

**Context (5 min)**
1. Which courses or groups do you take part in this semester, and how many people are in a typical group?
2. Which tools does your group use for chat, calls, files and deadlines? Who chose them?

**Current workflow (10 min)**
3. Walk me through the last time your group prepared a joint assignment. Where did each step happen?
4. How do you learn about a new assignment, a change of deadline or a room change?
5. How do you share and read documents (slides, PDFs, spreadsheets)? Do you download them to read?
6. How do online lessons or group calls work for you? What goes wrong?

**Pain points (8 min)**
7. What do you do because the tools do not work together?
8. Have you missed something important because it was in another tool? What happened?
9. _(Teachers)_ How do you collect submissions and return feedback today? How long does it take?
10. _(IT / client)_ What are the requirements for hosting, data storage and accounts at the university?

**Priorities (5 min)**
11. If you could keep only three features in one tool for your group, which would they be?
12. Show the prototype screens ([`08-ux-prototype.md`](08-ux-prototype.md)): what is unclear, what is missing?

## Hypotheses and how they are evidenced

Each requirement in [`06-requirements.md`](06-requirements.md) refers to the hypotheses below. A hypothesis moves to **Verified** only when at least two independent participants confirm it, or the client confirms it for client-side constraints.

| ID | Hypothesis | Supporting evidence so far | Status |
|---|---|---|---|
| H1 | Study groups combine at least three separate tools for chat, calls, files and deadlines. | Published studies of chat and LMS use in higher education ([`03-literature-review.md`](03-literature-review.md)) | _Assumption — to verify in interviews_ |
| H2 | Students miss announcements and deadline changes because they are posted in a different tool than the group discussion. | — | _Assumption — to verify in interviews_ |
| H3 | Reading shared documents without downloading them is valued by students and teachers. | — | _Assumption — to verify in interviews_ |
| H4 | Teachers need assignments with submission, feedback and grades inside the same group space as the discussion. | Teams for Education and Google Classroom both provide this ([`04-existing-systems.md`](04-existing-systems.md)) | _Partly evidenced (market); user verification pending_ |
| H5 | Online lessons need screen sharing and stable calls for 20–40 participants. | — | _Assumption — to verify in interviews_ |
| H6 | The university prefers a system it can host itself and that keeps student data on university servers. | — | _Assumption — to verify with the client_ |
| H7 | A desktop client and a usable mobile web view are both needed. | — | _Assumption — to verify in interviews_ |

## Findings

Planned. One row per finding, with the participants who stated it.

| ID | Finding | Participants | Related hypothesis | Verified? |
|---|---|---|---|---|
| F1 | Planned | Planned | Planned | Planned |

## Impact on requirements

Planned. Requirements added, changed or reprioritised after the interviews, and why.
