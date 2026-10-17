# Working in this repo

Read [README.md](README.md) first for the product, its entry points, and the
shortest path to run it. Read [development.md](development.md) before changing
code: it owns the architecture, game-rule principles, validation ladder,
visual-work rules, and contributor workflow. Active plans live under
[specs](specs/), and each plan says what to do next. If a folder has a README,
read it before continuing; that README owns the folder's details.

These are the durable agent principles:

- Lead with the contract when work crosses a component boundary. Say what the
  command, observation, publication, digest, fixture, or module contract is and
  how it changes.
- Answer routine questions from evidence. Ask only when the answer changes a
  consequential decision that the repository cannot settle.
- Optimize for the fastest trustworthy feedback. Run the narrowest check first,
  preserve real acceptance requirements, and run the full closeout suite when
  the implementation is complete.
- Keep one owner per concept. Reuse the existing owner before adding a type,
  adapter, rule, or helper; use [refactor-clean](.agents/skills/refactor-clean/SKILL.md)
  when a change exposes duplicate or stale ownership.
- Treat the simulation as authoritative and the player-facing result as a
  visual contract. Use the workflows linked from [development.md](development.md)
  for mechanics, UI, renderer, tests, and review.

Skills hold the procedures behind these principles. Load the skill that covers
the work before starting. Before changing this file, use
[audit-agents](.agents/skills/audit-agents/SKILL.md).
