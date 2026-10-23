# 3. Literature review

**Sources:** 21 in total, of which **17 are peer-reviewed**: 15 journal or conference papers and 2 book chapters. The other 4 are 1 W3C standard, 2 IETF RFCs and 1 documentation page. Every source was checked against its publisher page or the Crossref and OpenAlex records. Summaries are based on the abstracts or full texts that were read.

The review answers three questions:

1. What is known about how study groups collaborate online?
2. Which approaches exist for building real-time collaboration systems?
3. Which limitations must the project take into account?

## 3.1 Collaborative learning online

Computer-supported collaborative learning (CSCL) has a measurable effect. A meta-analysis of 143 studies in STEM education found a moderate overall effect size of 0.51, largest on learning processes [S1]. The same study shows that the effect depends on how the technology is combined with teaching practice, so a tool alone does not create collaboration. The classic review by Kreijns et al. [S2] names two pitfalls:

- assuming that social interaction happens just because the environment allows it;
- neglecting the social side: cohesion, trust and a sense of community.

Tan et al. [S3] moved a collaborative project-management activity onto Microsoft Teams and studied it with action research. They report that channels, shared files and meetings in one place can support active online collaboration.

**Implication for Groups:** the platform must support both task work (assignments, files, calendar) and informal social space (direct messages, reactions, drop-in voice rooms). Evaluation must measure learning-process outcomes, not only usage counts.

## 3.2 Chat and collaboration tools in teaching

Tuhkala and Kärkkäinen [S4] replaced email and the learning management system with Slack for all out-of-class communication in a thesis seminar, and studied help-seeking and ease of use. Wiles and Simmons [S5] used Discord in biology classes and labs. Students seemed more engaged with peers and instructors, and voice and text channels supported discussion outside scheduled class time. Both studies describe single courses and an instructor's own practice, without control groups, so they show feasibility rather than effect size.

**Implication:** persistent channels plus voice rooms are a workable model for study groups (FR-10, FR-11, FR-40). The weak evidence base is a reason to run our own user study (document 9) instead of assuming the effect.

## 3.3 Video meetings in education

Bailenson [S6] argues that video-meeting fatigue comes from "nonverbal overload", caused by interface features such as constant self-view and close-up faces. These are theoretical arguments that the author says remain to be tested. Fauville et al. [S7] turned the idea into the validated **Zoom Exhaustion & Fatigue (ZEF) Scale**, using 395 participants for development and 2 724 for validation. Fatigue rises with meeting frequency, duration and bursts of back-to-back meetings. Castelli and Sarvary [S8] found that students keep cameras off mainly because of concern about their appearance and their background, and because of weak internet. This may affect under-represented students more.

**Implication:** calls must work well with the camera off (audio-first voice rooms), offer background options and adapt to bandwidth (FR-40). The ZEF Scale is a ready instrument for the call part of the evaluation.

## 3.4 Acceptance of learning platforms

A systematic review of 71 studies [S9] confirms that perceived usefulness and perceived ease of use, the core of the Technology Acceptance Model (TAM), predict acceptance of learning technologies. A pandemic-era UTAUT study [S10] adds social influence and social isolation as predictors of intention to use a learning management system. A study of 974 students [S11] shows that facilitating conditions, meaning infrastructure and support, shape perceived ease of use and usefulness.

**Implication:**

- Usability is a requirement in its own right (NFR-USE-1, NFR-USE-2), and the evaluation includes TAM-style questions.
- Facilitating conditions support the self-hosted model: the university runs and supports the system itself.

## 3.5 Real-time media architecture

A conference with N participants can be built in three ways. These are the RTP topologies defined in RFC 7667 [S16]:

- **Peer-to-peer mesh** — every client sends its stream to every other client.
- **Mixer (MCU)** — a server decodes and mixes all streams into one.
- **Selective forwarding unit (SFU)** — a server forwards each stream without re-encoding.

The evidence on these topologies:

- **Bandwidth savings.** Grozev et al. [S13] show that selective forwarding of only the last N active speakers cut CPU use by 45 % and bandwidth by 63 % in a 30-participant conference.
- **SFU load testing.** André et al. [S12] load-tested five open-source SFUs and showed that systematic comparison is feasible, with differences in scalability between implementations.
- **Network behaviour.** Jansen et al. [S14] measured WebRTC under emulated and real networks: performance on wireless links was worse than expected.
- **Standards.** The browser API is standardised by the W3C [S15], and the protocol suite is described in RFC 8825 [S17].
- **LiveKit.** The SFU used in Groups forwards streams unchanged and adapts resolution and bitrate to each subscriber's bandwidth. It scales horizontally with Redis for room routing [S20].

**Implication:** an SFU is the right topology for class-sized calls (D4 in [`05-technology-comparison.md`](05-technology-comparison.md)). The evaluation must include a load test with a realistic number of participants and tests on a constrained wireless network (NFR-PERF-3).

## 3.6 Microservices and event-driven consistency

Dragoni et al. [S18] trace microservices back to service-oriented computing and list open problems. Laigner et al. [S19] combined a literature review, an analysis of open-source applications and a survey of more than 120 practitioners. They found that consistency between services, and the database-per-service pattern, are major unsolved data-management problems in practice. A 2026 systematic review [S21] groups consistency problems in event-driven microservices into three kinds — data, process and system evolution — and notes that they are usually treated in isolation.

**Implication:** separate databases per service need an explicit mechanism for reliable events. Groups uses a transactional outbox with ordered, de-duplicated publishing (NFR-REL-1). Event contracts are versioned to address system-evolution consistency. Both must be tested with fault injection, not only unit tests.

## 3.7 Structured analysis of sources

| ID | Source | Type | Theme | Method | Key findings | Limitations | Use in Groups |
|---|---|---|---|---|---|---|---|
| S1 | Jeong, Hmelo-Silver & Jo (2019) | Journal | CSCL | Meta-analysis, 143 studies, 316 outcomes | Moderate effect (0.51), strongest on process outcomes; moderated by technology and pedagogy | STEM only; studies up to 2014 | Group-centred design; evaluate process outcomes |
| S2 | Kreijns, Kirschner & Jochems (2003) | Journal | CSCL | Literature review | Social interaction is not automatic; social-psychological side is neglected | Old, conceptual | Social features next to task tools |
| S3 | Tan, Casanova, Huet & Alhammad (2022) | Journal | CSCL, tools | Action research, one module | Teams can support active online collaborative learning | One module, authors studied own teaching | Reference workflow: channels, files, meetings |
| S4 | Tuhkala & Kärkkäinen (2018) | Journal | Tools | Case study with questionnaire | Slack replaced email and LMS for seminar communication | One small course, pre-COVID | Channel chat for peer help |
| S5 | Wiles & Simmons (2022) | Journal | Tools | Practitioner report | Discord seemed to raise engagement; discussion outside class | No control group or sample size | Text channels plus voice rooms |
| S6 | Bailenson (2021) | Journal | Video | Theoretical argument | Nonverbal overload causes video fatigue | Untested by design | Hide self-view, audio-first rooms |
| S7 | Fauville et al. (2021) | Journal | Video | Scale development, N = 395 and 2 724 | Five fatigue dimensions; linked to frequency and duration | General adults, 2020 | ZEF Scale in the evaluation |
| S8 | Castelli & Sarvary (2021) | Journal | Video | Student survey, one course | Cameras off because of appearance, background, bandwidth | One course; exploratory subgroups | Camera-optional design, bandwidth adaptation |
| S9 | Granić & Marangunić (2019) | Journal | Acceptance | Systematic review, 71 studies | TAM credible; ease of use and usefulness confirmed | One database; up to 2018 | TAM questions in the evaluation |
| S10 | Raza, Qazi, Khan & Salam (2021) | Journal | Acceptance | Survey, PLS-SEM, extended UTAUT | Performance, effort, social influence and isolation predict LMS use | Pandemic context; sample not stated in abstract | Social features; UTAUT constructs |
| S11 | Sukendro et al. (2020) | Journal | Acceptance | Survey, N = 974, extended TAM | Facilitating conditions predict ease of use and usefulness | One country and discipline | Institution-run deployment and support |
| S12 | André et al. (2018) | Conference | Media | Load tests of 5 open-source SFUs | Systematic SFU comparison is feasible; scaling differs | 2018 versions; LiveKit not included | Load-test method for calls |
| S13 | Grozev, Marinov, Singh & Ivov (2015) | Conference | Media | SFU experiment vs full forwarding | −45 % CPU, −63 % bandwidth with last-N forwarding, 30 participants | Older; authors' own system | SFU for class-sized calls |
| S14 | Jansen et al. (2018) | Journal | Media | Emulated and real-network measurements | Wireless performance below expectations | 2018 codecs and browsers | Tests on constrained wireless networks |
| S15 | W3C WebRTC Recommendation (2025) | Standard | Media | Specification | Browser real-time communication API | API only, no server topology | Normative reference for clients |
| S16 | Westerlund & Wenger, RFC 7667 (2015) | Standard | Media | Informational RFC | Defines RTP topologies incl. selective forwarding middlebox | Descriptive | Terms for SFU vs MCU decision |
| S17 | Alvestrand, RFC 8825 (2021) | Standard | Media | Standards-track RFC | Overview of the WebRTC protocol suite | Overview only | Protocol background |
| S18 | Dragoni et al. (2017) | Book chapter | Architecture | Survey chapter | Microservices: origins, practice, open problems | Introductory | Rationale for service decomposition |
| S19 | Laigner et al. (2021) | Journal | Architecture | Literature review, code analysis, survey of 120+ | Cross-service consistency is a key unsolved practical problem | Self-selected survey | Outbox and event contracts |
| S20 | LiveKit documentation — SFU internals | Documentation | Media | Vendor documentation | Forwarding without re-encoding, per-subscriber adaptation, horizontal scaling | Not peer-reviewed, may change | Describes the SFU Groups runs |
| S21 | Bação & Guerreiro (2026) | Journal | Architecture | Systematic review | Data, process and evolution consistency treated in isolation | Very recent; library selection | Versioned contracts and fault tests |

## 3.8 Limitations the project takes into account

1. **Weak causal evidence for chat tools in teaching** [S4, S5]. Groups does not claim learning gains. It measures usability, acceptance and task performance, and compares them with a baseline (document 9).
2. **Video fatigue and camera reluctance** [S6–S8]. Calls are designed to work well without cameras, and fatigue is measured with the ZEF Scale.
3. **Acceptance depends on facilitating conditions** [S9–S11]. Deployment and support documentation are deliverables, not extras.
4. **SFU performance differs between implementations and networks** [S12–S14]. Call quality is measured on our own deployment, including on a wireless network, instead of relying on vendor claims.
5. **Consistency across services is hard** [S19, S21]. Reliability is verified with fault-injection tests.

## References

- [S1] Jeong, H., Hmelo-Silver, C. E., & Jo, K. (2019). Ten years of computer-supported collaborative learning: A meta-analysis of CSCL in STEM education during 2005–2014. *Educational Research Review, 28*, 100284. https://doi.org/10.1016/j.edurev.2019.100284
- [S2] Kreijns, K., Kirschner, P. A., & Jochems, W. (2003). Identifying the pitfalls for social interaction in computer-supported collaborative learning environments: A review of the research. *Computers in Human Behavior, 19*(3), 335–353. https://doi.org/10.1016/S0747-5632(02)00057-2
- [S3] Tan, C., Casanova, D., Huet, I., & Alhammad, M. (2022). Online collaborative learning using Microsoft Teams in higher education amid COVID-19. *International Journal of Mobile and Blended Learning, 14*(1), 1–18. https://doi.org/10.4018/IJMBL.297976
- [S4] Tuhkala, A., & Kärkkäinen, T. (2018). Using Slack for computer-mediated communication to support higher education students' peer interactions during Master's thesis seminar. *Education and Information Technologies, 23*(6), 2379–2397. https://doi.org/10.1007/s10639-018-9722-6
- [S5] Wiles, A. M., & Simmons, S. L. (2022). Establishment of an engaged and active learning community in the biology classroom and lab with Discord. *Journal of Microbiology & Biology Education, 23*(1), e00334-21. https://doi.org/10.1128/jmbe.00334-21
- [S6] Bailenson, J. N. (2021). Nonverbal overload: A theoretical argument for the causes of Zoom fatigue. *Technology, Mind, and Behavior, 2*(1). https://doi.org/10.1037/tmb0000030
- [S7] Fauville, G., Luo, M., Queiroz, A. C. M., Bailenson, J. N., & Hancock, J. (2021). Zoom Exhaustion & Fatigue Scale. *Computers in Human Behavior Reports, 4*, 100119. https://doi.org/10.1016/j.chbr.2021.100119
- [S8] Castelli, F. R., & Sarvary, M. A. (2021). Why students do not turn on their video cameras during online classes and an equitable and inclusive plan to encourage them to do so. *Ecology and Evolution, 11*(8), 3565–3576. https://doi.org/10.1002/ece3.7123
- [S9] Granić, A., & Marangunić, N. (2019). Technology acceptance model in educational context: A systematic literature review. *British Journal of Educational Technology, 50*(5), 2572–2593. https://doi.org/10.1111/bjet.12864
- [S10] Raza, S. A., Qazi, W., Khan, K. A., & Salam, J. (2021). Social isolation and acceptance of the learning management system (LMS) in the time of COVID-19 pandemic: An expansion of the UTAUT model. *Journal of Educational Computing Research, 59*(2), 183–208. https://doi.org/10.1177/0735633120960421
- [S11] Sukendro, S., Habibi, A., Khaeruddin, K., Indrayana, B., Syahruddin, S., Makadada, F. A., & Hakim, H. (2020). Using an extended Technology Acceptance Model to understand students' use of e-learning during Covid-19: Indonesian sport science education context. *Heliyon, 6*(11), e05410. https://doi.org/10.1016/j.heliyon.2020.e05410
- [S12] André, E., Le Breton, N., Lemesle, A., Roux, L., & Gouaillard, A. (2018). Comparative study of WebRTC open source SFUs for video conferencing. In *2018 Principles, Systems and Applications of IP Telecommunications (IPTComm)* (pp. 1–8). IEEE. https://doi.org/10.1109/IPTCOMM.2018.8567642
- [S13] Grozev, B., Marinov, L., Singh, V., & Ivov, E. (2015). Last N: Relevance-based selectivity for forwarding video in multimedia conferences. In *Proceedings of the 25th ACM Workshop on Network and Operating Systems Support for Digital Audio and Video (NOSSDAV '15)* (pp. 19–24). ACM. https://doi.org/10.1145/2736084.2736094
- [S14] Jansen, B., Goodwin, T., Gupta, V., Kuipers, F., & Zussman, G. (2018). Performance evaluation of WebRTC-based video conferencing. *ACM SIGMETRICS Performance Evaluation Review, 45*(3), 56–68. https://doi.org/10.1145/3199524.3199534
- [S15] Jennings, C., Castelli, F., Boström, H., Bruaroey, J.-I., et al. (2025). *WebRTC: Real-Time Communication in Browsers*. W3C Recommendation, 13 March 2025. https://www.w3.org/TR/webrtc/
- [S16] Westerlund, M., & Wenger, S. (2015). *RTP Topologies* (RFC 7667). IETF. https://www.rfc-editor.org/rfc/rfc7667
- [S17] Alvestrand, H. (2021). *Overview: Real-Time Protocols for Browser-Based Applications* (RFC 8825). IETF. https://www.rfc-editor.org/rfc/rfc8825
- [S18] Dragoni, N., Giallorenzo, S., Lafuente, A. L., Mazzara, M., Montesi, F., Mustafin, R., & Safina, L. (2017). Microservices: Yesterday, today, and tomorrow. In *Present and Ulterior Software Engineering* (pp. 195–216). Springer. https://doi.org/10.1007/978-3-319-67425-4_12
- [S19] Laigner, R., Zhou, Y., Salles, M. A. V., Liu, Y., & Kalinowski, M. (2021). Data management in microservices: State of the practice, challenges, and research directions. *Proceedings of the VLDB Endowment, 14*(13), 3348–3361. https://doi.org/10.14778/3484224.3484232
- [S20] LiveKit. (n.d.). *LiveKit SFU*. Documentation, retrieved 2026-10-23. https://docs.livekit.io/reference/internals/livekit-sfu/
- [S21] Bação, J. L., & Guerreiro, S. (2026). Consistency challenges in event-driven microservices: A literature review on data, process, and system evolution. *Computing, 108*(7), 110. https://doi.org/10.1007/s00607-026-01701-5
