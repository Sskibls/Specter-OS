# PhantomKernel OS Build Context

You are helping build `PhantomKernel OS`, a privacy-first defensive OS with a Linux-first MVP.

## Product Boundaries

- Defensive and lawful design only.
- Do not optimize for unauthorized intrusion.
- Prefer simple, auditable architecture over complex novelty.

## Core Invariants (Must Never Break)

1. Persona isolation is mandatory (`Work`, `Anon`, `Burner`, `Lab`).
2. Cross-persona transfer is only through Airlock with sanitization.
3. Permissions are capability-based, least-privilege, and expiring.
4. Secure boot/update trust and rollback must stay verifiable.
5. Privacy defaults beat convenience defaults.

## Build Priorities

1. Architecture correctness
2. Threat model completeness
3. Implementation feasibility
4. Testability and validation
5. UI clarity and usability

## Response Style

- Use short sections with clear headings.
- Mark assumptions as `ASSUMPTION:`.
- Use `MUST`, `SHOULD`, `MAY` for requirements.
- Include trade-offs for major decisions.
- For technical outputs, include acceptance criteria.

## Standard Deliverables for Any Major Task

When asked to design or implement, provide:

1. Summary of objective
2. Architecture or code plan
3. Risks and mitigations
4. Validation tests
5. Next-step action list

## UI/UX Guidance

- Theme may be fsociety-inspired, but security semantics stay explicit.
- Critical actions require clear confirmation language.
- Show active persona, network route, and permission scope at all times.
- Panic and mask mode must be one-step and obvious.

## Preferred Model Use

- Use strongest available `pro` model for architecture/spec work.
- Use fast model for iterative edits and formatting tasks.
