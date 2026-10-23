# 1. Project proposal and scope

## Project title

**Groups — a self-hosted collaboration platform for study groups**

## Team

| Member | GitHub | Role in the project |
|---|---|---|
| Azarenkov Alexey | [@azarenkov](https://github.com/azarenkov) | Team lead, backend and infrastructure: Rust services, database schemas, messaging, real-time media, deployment |
| Alihan Manat | [@alishahmanov](https://github.com/alishahmanov) | Frontend: chat, channels, threads, reactions, presence, rich text |
| Iskander Akipov | [@Iskander1612](https://github.com/Iskander1612) | Frontend: assignments, calendar and meetings, files, notifications, document viewer |

**Supervisor:** Zhaksylyk Kozhakhmet ([@zhaks14](https://github.com/zhaks14))

Responsibilities are traced to tasks in [`10-project-management.md`](10-project-management.md).

## Problem statement

University study groups run their daily work across several unconnected tools: a messenger for chat, a video service for calls, a learning management system for assignments, and cloud storage for files. Teachers repeat announcements in several places, students miss deadlines that live in a different tool than the discussion, and documents are downloaded just to be read. The integrated alternatives (Microsoft Teams for Education, Google Workspace for Education) are proprietary cloud services: the university cannot host them on its own servers, cannot control where student data is stored, and depends on licence terms that it does not set. A detailed, evidence-based comparison is in [`04-existing-systems.md`](04-existing-systems.md).

The project addresses this gap with one self-hostable system that combines group communication, real-time meetings and the core study workflow.

## Target users

| User | Needs |
|---|---|
| **Student** | One place for group chat, calls, files, deadlines and grades; works in the browser, on desktop and on a phone |
| **Teacher** | Create groups for a class, post announcements, run online lessons with screen sharing, publish assignments and grade submissions |
| **University IT administrator** | Deploy and operate the system on university infrastructure, keep data on own servers, scale it with the number of users |

## Aim

Design and implement a self-hostable, Teams-like collaboration platform for study groups that covers communication (chat, calls), study workflow (assignments, meetings, announcements) and document work in one system, and that a university can run on its own infrastructure.

## Objectives

1. Analyse how study groups collaborate and which tools they use (interviews, literature, existing systems) — documents 2–4.
2. Specify functional and non-functional requirements with priorities and acceptance criteria — document 6.
3. Design a service-oriented architecture with clear bounded contexts, reliable event delivery and real-time media — document 7.
4. Implement the communication core: groups and channels, direct messages, threaded replies, reactions, attachments with an in-app document viewer, presence, voice and video calls with screen sharing.
5. Implement the study workflow: announcements and teacher-only posting, assignments with submission and grading, scheduled meetings with a group calendar, notifications.
6. Provide deployment for a single server (Docker Compose) and for a cluster (Kubernetes with Helm and Terraform).
7. Verify the result with automated tests, load and latency measurements and a usability study — document 9.

## Expected outcomes

- A working web application, a desktop application (Electron) and responsive mobile web views.
- Back-end services with automated tests and a documented deployment for one server and for Kubernetes.
- A documented architecture, requirements traceable to tasks and tests, and an evaluation report against the metrics in document 9.

## Scope

**In scope (this diploma project):**

- Accounts with student and teacher roles, email verification, password reset.
- Groups (classes) with channels, roles and invitations; direct messages.
- Messaging: text with formatting, threaded replies, reactions, mentions, pins, attachments, stickers, read state, search.
- In-app viewer for PDF, Word, PowerPoint, Excel and text files.
- Voice and video calls in channels and direct messages, screen sharing.
- Announcements, assignments with submission and grading, scheduled meetings and a group calendar, push notifications.
- Self-hosted deployment with Docker Compose and Kubernetes.

**Planned if time allows (priority P1 and P2 in the task board):** polls, a files tab, meeting controls, presence states, saved messages and scheduled send, join by class code, recordings and captions, breakout rooms, attendance reports, whiteboard, class notes, message translation.

**Out of scope:** integration with university student information systems, payment, plagiarism checking, native mobile applications (the mobile experience is the responsive web app).

## Engineering complexity

The project is not a CRUD application. It includes:

- Six back-end services and two workers in Rust with separate databases per service and a strict layering (domain, use cases, ports, adapters).
- Event-driven integration through NATS JetStream with a transactional outbox, per-conversation ordering and idempotent consumers.
- Real-time delivery to clients over WebSockets, and real-time media through a LiveKit SFU (WebRTC).
- A sandboxed document conversion service for office files.
- Deployment automation: Docker images, Helm chart, Terraform, network policies.

Current status and the full feature list are in the [README](../README.md#current-status).
