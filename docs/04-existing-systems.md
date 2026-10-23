# 4. Existing system comparison

**Systems:** Microsoft Teams for Education, Google Classroom (with Google Meet and Google Chat), Moodle, Slack, Discord. Teams and Classroom are the main integrated education platforms. Moodle is the most widely deployed open-source learning management system. Slack and Discord are used by study groups for chat (literature sources S4 and S5 in [`03-literature-review.md`](03-literature-review.md)).

**Criteria:** 10 criteria derived from the target users' needs in [`01-project-proposal.md`](01-project-proposal.md#target-users).

**Evidence:**

- Every cell refers to the vendor's own documentation, help centre or pricing page, all accessed on 23 October 2026.
- "Not found" means no official page stating the capability was found. It is not a confirmed "No".
- `docs.moodle.org` blocked automated access, so the Moodle claims rely on Moodle release notes, moodle.com, the Moodle Marketplace, MoodleCloud and the official repository.

## Comparison

| Criterion | Microsoft Teams (Education) | Google Classroom (+ Meet, Chat) | Moodle | Slack | Discord | **Groups** |
|---|---|---|---|---|---|---|
| 1. Group channels, real-time chat | Yes — up to 1 000 channels per team; group chats up to 250 people [T1] | Partial — the Classroom stream has text-only comments, not chat [G3]; chat needs separate Google Chat spaces [G1] | Partial — 1:1 and group messaging [M3]; the Chat activity was removed from core in 5.0 [M2] | Yes [SL1] | Yes — up to 500 channels per server [D5] | **Yes** — implemented |
| 2. Threads and reactions | Yes — replies attached to channel posts [T2]; reactions [T3] | Partial — replies to comments only, on computer [G3]; threads and reactions in Google Chat [G1][G2] | Partial — forum replies [M4]; reactions in core not found | Yes [SL2][SL3] | Yes [D1][D2] | **Planned** — #2, #3 (P0) |
| 3. Voice/video and screen sharing; limits | Yes — up to 300 on A1, 1 000 on A3/A5; calls from chat up to 20 [T1] | Yes, via Meet — 100 on the free Fundamentals edition, up to 1 000 on Education Plus [G4]; screen sharing [G5] | Yes, via BigBlueButton plug-in [M6]; free hosted tier 25 participants, 1 hour [M7] | Huddles — 2 on Free, up to 50 on paid plans [SL4] | Yes — video up to 25, screen share up to 50 [D3][D4][D5] | **Yes** — implemented on a self-hosted SFU; target 30 participants (NFR-PERF-3) |
| 4. Assignments with submission and grading | Yes — turn in, grade, return [T4]; up to 1 000 members per class team [T1] | Yes [G7][G8] | Yes [M8] | Not found | Not found | **Planned** — #4 (P0) |
| 5. Scheduled meetings, calendar | Yes — Teams calendar, Outlook sync [T5] | Yes — Google Calendar; Classroom calendar read-only [G9] | Yes — core calendar [M5] | Partial — Google Calendar integration [SL5] | Partial — Scheduled Events, voice events capped at 99 [D6] | **Planned** — #5 (P0) |
| 6. In-app preview of Office and PDF | Yes — PDF, DOCX, PPTX, XLSX [T6] | Yes, via Drive [G10] | Partial — PDF annotation of submissions [M9]; general Office preview not found | Partial — no preview for Office files over 50 MB [SL6] | No — PDF and DOC show as embeds [D7]; text preview up to 50 KB [D8] | **Yes** — implemented (#21) |
| 7. Self-hosting, licence | No — on-premises only as legacy Skype for Business Server [T7]; proprietary | Not found (SaaS); proprietary | Yes — GPL-3.0 [M1] | Not found (cloud plans only [SL7]); proprietary | Not found; proprietary | **Yes** — every component self-hosted, open-source dependencies |
| 8. Education pricing | Office 365 A1 free for eligible institutions [T8] | Education Fundamentals free; Plus paid [G11] | Free software [M1]; MoodleCloud from US$170/year for 50 users [M10] | Free plan with 90-day history [SL7]; 85 % education discount on paid plans [SL8] | Free [D9]; Nitro optional [D10] | **Free**; cost is the university's own infrastructure |
| 9. Data residency, control | Tenant geography; choosing a region needs paid add-ons [T9] | Data regions on Education Standard and Plus only [G12] | Full control when self-hosted [M1] | US by default; regional residency on Business+ and Enterprise only [SL9] | US and other countries; no choice [D11] | **Full control** — data stays on university servers (NFR-PRIV-1) |
| 10. Desktop and mobile clients | Windows, macOS, web, iOS, Android [T10] | Classroom: iOS, Android, web [G13] | Mobile app (free tier limited); desktop through the web [M12] | Mac, Windows, Linux, iOS, Android [SL10] | Windows, macOS, Linux, iOS, Android, web [D12] | **Web, desktop (Electron) for Windows, macOS, Linux; responsive mobile web**; no native mobile apps |

## Limitations of existing systems for a university study group

**Microsoft Teams for Education**
- Not self-hostable: moving to Teams means moving users to the cloud [T7]. Choosing the data region needs paid add-ons [T9].
- Calls started from a chat are capped at 20 people, and group chats over 20 members lose calling and read receipts [T1].
- Class teams require an Office 365 Education licence [T1].

**Google Classroom (+ Meet, Chat)**
- The class stream is not a chat. Discussion needs a separate product, Google Chat [G3][G1].
- The free edition caps meetings at 100 participants [G4] and offers no data-region control [G12].
- The Classroom calendar is read-only [G9].

**Moodle**
- Real-time group chat was removed from core in Moodle 5.0 [M2].
- Video depends on BigBlueButton. The free hosted tier is limited to 25 participants and one hour [M7]; otherwise the institution runs its own BigBlueButton server.
- No general Office preview was found [M9].

**Slack**
- The free plan keeps 90 days of history and allows only 1:1 huddles [SL7][SL4].
- No assignments or grading [SL7]. The education discount applies only to paid plans [SL8].
- Data is stored in the US unless the workspace is on Business+ or Enterprise [SL9].

**Discord**
- No preview of PDF or Office files [D7][D8].
- Video is limited to 25 participants per voice channel [D3].
- No education features. Data is stored in the US and other countries with no choice of region [D11].

## Why Groups is needed

- **No proprietary option is self-hostable.** None of the integrated systems with chat, meetings and assignments (Teams, Classroom) can be hosted by the university [T7], and both tie data-region control to paid editions [T9][G12].
- **Self-hostable Moodle lacks real-time collaboration.** It has assignments and a calendar, but no core real-time chat [M2] and no built-in meetings without BigBlueButton [M6].
- **Chat-first tools lack the study workflow.** Slack and Discord have good chat and calls, but no assignments, grading or document preview, and they keep data in the US [SL9][D11].

Groups combines:

- the communication model of Teams, Slack and Discord: channels, threads, reactions, calls and screen sharing;
- the study workflow of Teams, Classroom and Moodle: assignments, calendar, announcements;
- in-app document preview;
- complete self-hosting with data under university control.

**Where Groups is behind today:**

- threads, reactions, assignments and the calendar are planned but not yet implemented (see [`06-requirements.md`](06-requirements.md));
- there are no native mobile apps;
- the commercial systems support far larger scale than the 30-person call target.

## References

All accessed 23 October 2026.

**Microsoft**
- [T1] Limits and specifications for Microsoft Teams — https://learn.microsoft.com/en-us/microsoftteams/limits-specifications-teams
- [T2] Send or reply to a channel message in Microsoft Teams — https://support.microsoft.com/en-us/teams/teams-channels/send-or-reply-to-a-channel-message-in-microsoft-teams
- [T3] Chat in Microsoft Teams meetings — https://support.microsoft.com/en-us/teams/meetings/chat-in-microsoft-teams-meetings
- [T4] Assignments and grades in your class team — https://support.microsoft.com/en-us/education/teams/assignments-and-grades-in-your-class-team
- [T5] Schedule a meeting in Microsoft Teams — https://support.microsoft.com/en-us/teams/meetings/schedule-a-meeting-in-microsoft-teams
- [T6] File types supported for previewing files in OneDrive, SharePoint, and Teams — https://support.microsoft.com/en-us/onedrive/file-types-supported-for-previewing-files-in-onedrive-sharepoint-and-teams
- [T7] Upgrade Skype for Business on-premises to Microsoft Teams — https://learn.microsoft.com/en-us/microsoftteams/upgrade-to-teams-execute-skypeforbusinesshybridonprem
- [T8] Office 365 Education — https://www.microsoft.com/en-us/education/products/office
- [T9] Data Residency for Microsoft Teams — https://learn.microsoft.com/en-us/microsoft-365/enterprise/m365-dr-service-teams
- [T10] System requirements for the Teams client — https://learn.microsoft.com/en-us/microsoftteams/teams-client-system-requirements

**Google**
- [G1] Use threads in Chat — https://support.google.com/chat/answer/12176589
- [G2] Reply to a message in Google Chat — https://support.google.com/chat/answer/7654374
- [G3] Post to the class stream — https://support.google.com/edu/classroom/answer/6020274
- [G4] Google Meet participant limits — https://support.google.com/meet/answer/7317473
- [G5] Present during a video meeting — https://support.google.com/meet/answer/9308856
- [G7] Create an assignment — https://support.google.com/edu/classroom/answer/6020265
- [G8] Grade and return an assignment — https://support.google.com/edu/classroom/answer/6020294
- [G9] View due dates and events in a calendar — https://support.google.com/edu/classroom/answer/6272985
- [G10] Preview files in Google Drive — https://support.google.com/drive/answer/2423485
- [G11] Compare Google Workspace for Education editions — https://edu.google.com/workspace-for-education/editions/compare-editions/
- [G12] Choose a geographic location for your data — https://knowledge.workspace.google.com/admin/compliance/choose-a-geographic-location-for-your-data
- [G13] Get the Classroom app — https://support.google.com/edu/classroom/answer/6118412

**Moodle**
- [M1] moodle/moodle repository (GPL v3) — https://github.com/moodle/moodle
- [M2] Moodle 5.0 release notes — https://moodledev.io/general/releases/5.0
- [M3] Moodle 3.6 release notes — https://moodledev.io/general/releases/3.6
- [M4] Moodle 3.7 release notes — https://moodledev.io/general/releases/3.7
- [M5] Moodle 3.3 release notes — https://moodledev.io/general/releases/3.3
- [M6] BigBlueButton plug-in — https://marketplace.moodle.com/plugins/mod_bigbluebuttonbn
- [M7] BigBlueButton free-tier hosting requires credentials — https://moodle.com/news/moodle-certified-integration-bigbluebutton-now-requires-credentials-for-their-free-tier-hosting/
- [M8] Moodle for vocational education (assignments) — https://moodle.com/solutions/vocational-training/
- [M9] PDF submission plug-in — https://marketplace.moodle.com/plugins/assignsubmission_pdf
- [M10] MoodleCloud standard plans — https://www.moodlecloud.com/standard-plans/
- [M12] Moodle App — https://moodle.com/products/moodle-app/

**Slack**
- [SL1] What is a channel? — https://slack.com/help/articles/360017938993-What-is-a-channel
- [SL2] Use threads to organize discussions — https://slack.com/help/articles/115000769927
- [SL3] Use emoji and reactions — https://slack.com/help/articles/202931348-Use-emoji-and-reactions
- [SL4] Use huddles in Slack — https://slack.com/help/articles/4402059015315-Use-huddles-in-Slack
- [SL5] Google Calendar for Slack — https://slack.com/help/articles/206329808-Google-Calendar-for-Slack
- [SL6] Add files to Slack — https://slack.com/help/articles/201330736-Add-files-to-Slack
- [SL7] Slack pricing — https://slack.com/pricing
- [SL8] Slack for Education discount — https://slack.com/help/articles/206646877-Apply-for-the-Slack-for-Education-discount
- [SL9] Data residency for Slack — https://slack.com/help/articles/360035633934-Data-residency-for-Slack
- [SL10] Slack downloads — https://slack.com/downloads

**Discord**
- [D1] Threads FAQ — https://support.discord.com/hc/en-us/articles/4403205878423-Threads-FAQ
- [D2] Reactions FAQ — https://support.discord.com/hc/en-us/articles/12102061808663-Reactions-and-Super-Reactions-FAQ
- [D3] Video Calls — https://support.discord.com/hc/en-us/articles/360041721052-Video-Calls
- [D4] Go Live and Screen Share — https://support.discord.com/hc/en-us/articles/360040816151-Go-Live-and-Screen-Share
- [D5] Account Caps, Server Caps, and More — https://support.discord.com/hc/en-us/articles/33694251638295-Discord-Account-Caps-Server-Caps-and-More
- [D6] Scheduled Events — https://support.discord.com/hc/en-us/articles/4409494125719-Scheduled-Events
- [D7] File Attachments FAQ — https://support.discord.com/hc/en-us/articles/25444343291031-File-Attachments-FAQ
- [D8] File Preview — https://support.discord.com/hc/en-us/articles/1500005466681-File-Preview
- [D9] Getting Started — https://support.discord.com/hc/en-us/articles/360033931551-Getting-Started
- [D10] Discord Nitro — https://discord.com/nitro
- [D11] Discord Privacy Policy — https://discord.com/privacy
- [D12] Download Discord — https://discord.com/download
