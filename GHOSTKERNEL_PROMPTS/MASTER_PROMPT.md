You are the Lead Architect for a real, buildable privacy OS called PhantomKernel OS.

Goal:
Design a production-grade, defensive, privacy-first desktop OS with fsociety-style UX (visual theme only), stronger privacy defaults than mainstream security distros, and practical daily usability.

Non-negotiable constraints:
1) Defensive + lawful design only. No unauthorized-intrusion optimization.
2) Linux-first MVP (180 days), with clean migration path to microkernel-style service boundaries.
3) Persona isolation is core: Work, Anon, Burner, Lab (hard separation of storage/network/process context).
4) Capability-based permissions only (time-bound, context-bound, least privilege).
5) Cross-persona transfer only through an Airlock service with mandatory metadata sanitization.
6) Secure boot + signed updates + rollback + tamper-evident audit trail.
7) Local-first OPSEC assistant and leak simulator (no cloud dependency required).
8) Usability matters: must be daily-driver practical.

Required output (in this exact order):
A) Executive Summary (10 bullets max)
B) Product Requirements Document (actors, jobs-to-be-done, success metrics)
C) Threat Model (assets, adversaries, attack trees, STRIDE/LINDDUN mapping)
D) Architecture Spec (daemons, trust boundaries, IPC, data flow, failure modes)
E) Crypto + Key Management Spec (algorithms, key hierarchy, rotation/recovery)
F) Module Contracts for:
   - phantomkernel-init
   - phantomkernel-policyd
   - phantomkernel-shardd
   - phantomkernel-netd
   - phantomkernel-airlockd
   - phantomkernel-updated
   - phantomkernel-auditd
   - phantomkernel-guardian
   - phantomkernel-shell
G) API definitions (request/response schemas + authz checks + error codes)
H) 180-day implementation roadmap (biweekly milestones, dependencies, critical path)
I) Test & Validation Plan (unit/integration/e2e/security/leak/isolation tests)
J) Release Criteria (Definition of Done + go/no-go checklist)
K) Risk Register (top 20 risks + mitigations + owners)
L) Open questions (only truly blocking questions)

Quality bar:
- Mark every assumption as: ASSUMPTION: ...
- Use RFC-2119 language for requirements (MUST/SHOULD/MAY).
- Include trade-off tables for major design choices.
- Prefer simple, auditable designs over clever complexity.
- No hand-wavy statements; every claim needs an implementation path.

Formatting:
- Markdown only.
- Clear section headers matching A–L.
- Use concise tables where useful.
- End with “First 14-day execution sprint plan” with exact tasks.

Now produce the full specification.
