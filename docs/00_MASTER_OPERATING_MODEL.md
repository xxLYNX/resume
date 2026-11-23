---
title: Employment Acquisition System — Master Operating Model
created: 2026-11-06
status: proposed
sources:
  - 01_Employment_Acquisition_Strategy(1).md
  - 02_Targeted_Application_Playbook(1).md
  - CrowdStrike_R29279_Targeted_Application_Campaign(1).md
  - cursor-agent-undertakings-report.md
  - portfolio-export-for-agent.txt
  - https://github.com/xxLYNX/resume
---

# Employment Acquisition System — Master Operating Model

## Mission and invariants

**Mission:** Obtain rewarding technical employment while building an accurate, reusable map of capabilities and evidence. Companies and titles are *targets*, not identities. Every serious target must leave either a useful application, a new demonstrated capability, a reusable artifact, or a better estimate of which work is personally rewarding. A campaign may be stopped when its expected value collapses.

**Foundational distinction:**

- *Interest* = what work you want to do, including uncertainty and exploration.
- *Capability* = what you can reliably implement, debug, explain, or operate.
- *Claim* = a narrowly worded, falsifiable assertion about your contribution/capability.
- *Evidence* = a traceable observation, artifact, or outcome supporting the claim.
- *Requirement* = an employer's expressed or inferred expectation, not automatically an objective test of competence.
- *Campaign* = a time-bounded effort to pursue a posting and produce relevant evidence.
- *Activity* = a completed action whose output can be checked.
- *Outcome* = an observed real-world result, such as a screen, interview, offer, or published research artifact.

Never conflate **specified, implemented, tested, deployed, observed, and maintained**. Distinguish a transcript reporting an action from inspecting its actual outcome. Treat both gaps and uncertain history as first-class information.

## System boundaries — do not build a new universe

| Surface                                             | Owns                                                                                                                                                       | Does not own                                                              |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| **JobDesk (SQLite)**                                | Portfolio anchors; capability claims, evidence index, requirement and campaign records; applications; résumé snapshots; screening outcomes; decision queue | Source-code implementation, host runtime truth, agent free-form reasoning |
| **Project repositories** (Forgejo / GitHub / local) | Code, tests, commits, design decisions, issue history, reproducible results                                                                                | Personal labor-market planning or entire résumé state                     |
| **NEBULA**                                          | Desired declarative host/system configurations and deployment mechanism                                                                                    | Proof that deployed controls are currently operating                      |
| **CONSTELLATION** (intended)                        | Observed inventory/posture, evidence/provenance of operational state, deviations from desired state                                                        | Replacement for NEBULA or JobDesk                                         |
| **Obsidian / Oracle**                               | Human-readable research, hypotheses, concepts, learning notes and links                                                                                    | Authoritative claims of live deployment or actual applications            |
| **External job postings and ATS**                   | Employer requirements and external application outcomes                                                                                                    | Internal confidence that the user possesses capabilities                  |

Maintain references between systems rather than copying every document into JobDesk. Stable identifiers and source locators are the important primitives.

## Workflow A — Discover and triage a role

1. Capture source URL, posting text or immutable snapshot, requisition, location, work mode, compensation/constraints, timestamp and expiration uncertainty.
2. Infer the daily activities. Mark each as **explicit** or **inferred** and rank its centrality.
3. Apply hard constraints: legal eligibility, required current clearance where genuinely mandatory, onsite/relocation, compensation floor, schedule, deadline. *Willingness to obtain a clearance does not equal possession of an active clearance.*
4. Rate **Personal Desirability** and **Hiring Feasibility** separately. A role can be enjoyable yet implausible now; feasible yet draining.
5. Build a requirement–claim matrix with statuses: verified direct evidence, under-evidenced, adjacent, acquirable, structural gap, hard constraint, unknown.
6. Decide: **APPLY NOW**, **TIMEBOXED PREP**, **EXPLORE**, or **PASS**. Record the reason and next re-evaluation trigger.
7. Use the existing playbook for every selected target, not as a blank document you regenerate.

### Personal desirability — a vector, not one opaque score

Use a 1–5 self-rating on *anticipated daily activities*, plus a 1–5 confidence rating for each prediction. At least initially, track five independent dimensions:

- **Task energy:** Would I willingly return to the core work tomorrow? Use concrete verbs and context: debugging concurrent code; interpreting logs; synthesizing research; reviewing a compliance binder; resolving customer incidents.
- **Work environment:** Autonomy, mentorship, feedback quality, interruption load, collaboration, on-call cadence and constraints.
- **Exploration value:** Does the role let me discover an interesting domain through actual work? Distinguish *intriguing unknown* from *vague job description*.
- **Option value:** Skills, connections and stories useful across multiple role families.
- **Life compatibility:** Geography, work arrangement, compensation, schedule, ethics and stability.

Avoid summing uncalibrated ratings blindly. A highly rated task cannot cancel a hard constraint or an intolerable on-call policy. Use a two-axis **desirability × feasibility** portfolio, with uncertainty attached. Once a role is experienced, compare predictions with interviews, informational conversations, or practical work.

### Career hypotheses to test, not permanent identities

1. **Endpoint / security systems engineering:** operating systems, telemetry, C/C++, concurrency, debugging, performance and endpoint protection.
2. **Vulnerability research / reverse engineering:** root-cause analysis, crash triage, exploitability bounds, validation against patched builds, technical communication.
3. **Security platform / detection engineering:** declarative fleet management, identity/policy enforcement, observability, incident investigations, infrastructure and software supply chain.
4. **Research-oriented software / developer tooling:** FRACTAL, JobDesk, orchestration and information systems, with emphasis on architecture and owned implementation.

An application can draw on multiple hypotheses. Re-rank using actual exposure, not the novelty of its technology list.

## Workflow B — Mine the existing portfolio

For each candidate accomplishment:

1. **Discover:** from repo, report, commit, log, benchmark, screenshot, paper, or personal recollection. Import history as a *lead*, not a verified fact.
2. **Decompose:** break a project description into independently testable claims (one claim ≠ one project).
3. **Locate evidence:** pin commit and path, test output and environment, comparison data, deployed revision, incident notes, stakeholder feedback, publication, or a repeatable demo.
4. **Challenge:** independent reproduction; live code modification; attack the measurement methodology; determine exactly what AI authored; identify unimplemented or merely designed parts.
5. **Classify:** `candidate → supported → verified → publication_ready`; allow `disputed`, `superseded`, and `retracted`. Record *who* verified *what* and *when*.
6. **Package:** a compact evidence packet (problem, constraints, role, implementation, observed result, tradeoffs, failure, links). A recruiter-facing version omits private IPs, internal network topology, credentials and unreleased proprietary details.
7. **Reuse:** link claims to relevant requirements across campaigns. Do not duplicate the claim for each résumé.

### Capability depth rubric (human-audited)

| Depth | Exit criterion |
|---|---|
| 0 — unknown | No meaningful claim possible |
| 1 — oriented | Can explain main concepts; no independent implementation |
| 2 — practiced | Can implement a small exercise with references |
| 3 — independent | Can build, modify and debug a nontrivial example |
| 4 — operational | Has handled real failure modes, validated results, and can explain tradeoffs |
| 5 — deep | Can transfer/reason across contexts, teach, defend adversarial review, and reproduce complex work |

Depth is **context-dependent**, not permanent or a certification. A claim must list the scope under which it was observed. Never infer 4 or 5 merely from years spent using a tool.

## Workflow C — Gap acquisition

For a closeable requirement, use the existing immersion loop: **orient → retrieve → implement → break → debug → measure → explain → record**. Define the output and stop condition first. Prefer the smallest artifact that makes the missing capability interview-defensible; don't build a product merely because the spec inspires one.

**Example:** A portable C++ bounded event queue with measured overload/shutdown behavior can transfer to endpoint agents, high-throughput services, observability pipelines and systems roles. Native macOS collection and Swift interop are useful follow-ons *only when macOS targets remain valuable and Mac access exists*.

## Workflow D — Submission and interview

- Keep a general source résumé; compose target-specific versions from approved claims and pinned revisions.
- For every application, preserve original posting, material requirements, applied date, résumé actually sent, optional answers, referral/outreach, stage and outcome.
- **Never mark an external submission as an immutable JobDesk submission unless a frozen manifest actually exists.** For legacy applications, store an `external_submission` record with `snapshot_missing` if the document cannot be recovered.
- Apply when credibility crosses the threshold; do not wait for hypothetical perfection or suspend other viable targets.
- Translate résumé bullets into probable adversarial follow-ups, prepare actual bug/decision/performance stories, and practice from memory.
- After outcome, write back *which stage failed*, whether a skill claim was challenged, the observed signal and the next action. A rejection without feedback is not proof of a specific skill gap.

## Prioritization and anti-sprawl

Prioritize work on three independent axes:

1. **Employment leverage:** likelihood it changes the quality or availability of interviews soon.
2. **Transfer leverage:** number of appealing role families that benefit from the result.
3. **Evidence leverage:** how much credible proof the effort generates from work already done.

Divide by time/cost and account for uncertainties rather than assigning fake decimal probabilities. A public write-up of a *verified* research result may beat building a new tool for three weeks. A strong standalone JobDesk feature may itself become a portfolio artifact, but it should not displace applications indefinitely.

**Work-in-progress limit:** one central capability acquisition, one evidence-packaging track, and a narrow JobDesk infrastructure track at a time; application triage remains ongoing. Everything else stays discoverable but parked.

## Operating cadence

**Daily (15 minutes):** inspect the campaign queue; pick one next action and one explicit stop/exit condition. **At the end:** capture result, evidence locator, unresolved blockers and a short energy/interest observation.

**Weekly:** triage fresh roles; carry out one or two high-quality applications when credible opportunities exist; close at least one claim/verification gap; review stalled WIP; update job-family interest hypotheses. Measure outcomes, not number of browser tabs or hours studying.

**Monthly:** revisit role-family hypotheses, skill demand across *attractive* postings, opportunity cost, screen rates, portfolio evidence coverage, and whether JobDesk feature work is actually reducing friction.

### Guardrail metrics

- Percent of top résumé claims linked to at least one independently inspectable artifact.
- Percent of active-application **critical requirements** with verified, adjacent or explicitly acknowledged missing evidence.
- Time from role discovery to go/no-go and to credible submission.
- Time to reach an explicit capability exit condition; durability on independent retest.
- Recruiter-screen and technical-screen conversion, conditioned on realistic sample size.
- Reuse of a proof packet across multiple distinct target requirements.
- Time spent maintaining the system versus producing applications and evidence.
- Recorded changes in predicted versus experienced task enjoyment.

Do not optimize raw application count, commits, imported taxonomy nodes, résumé bullet count, or study hours.

## Next decision

**Start with truth restoration and a small amount of evidence population, not a new AI matching engine.** The system already has an application analyzer; it lacks enough reliable data for that analyzer to be strategically useful. The attached `01_PORTFOLIO_AUDIT.md`, `02_JOBDESK_EVIDENCE_SLICE_SPEC.md`, and `03_FIRST_TWO_WEEKS.md` put this operating model into an executable order.
