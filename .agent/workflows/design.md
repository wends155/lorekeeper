---
description: Iterative UI/UX design with mockup review loop (Pre-Think Phase)
---

# Design Workflow

This workflow defines the standard process for designing user interfaces
and visual assets **before** any implementation begins. It is the only
workflow with an **iterative review loop** — the user and agent go back
and forth on mockups until each screen is approved.

> [!IMPORTANT]
> This workflow is **design-only** — no code edits, no implementation.
> The output is a **Design Spec** (`design/design-spec.md`) that feeds
> into `/plan-making`.

> [!NOTE]
> The Design Spec produced here is the **input artifact** for `/plan-making`.
> The implementation plan references specific mockups for each component.

## Trigger

User invokes: `/design <description>`

## Prerequisites

- Read `.agent/rules/design-rules.md` for design modes, spec format, mockup conventions, and review protocol.
- Read `architecture.md` (if present) for project structure and module boundaries.
- Read `design/design-spec.md` (if present) for existing designs (revision mode).
- Confirm you are operating as the **Architect** role.

## Steps

### 1. Gather Design Intent

Determine the scope:

- **New design** or **revision** of an existing Design Spec?
- If revision: load existing `design/design-spec.md`, identify which screens/assets to revise.

Determine the design mode per `design-rules.md` §1:

| Mode | When |
|------|------|
| **GUI** | Web interfaces, desktop apps |
| **TUI** | Terminal UIs (Ratatui, crossterm, etc.) |
| **CLI** | Command-line output format |
| **Assets** | Favicon, app icon, logo, splash screen |

Gather requirements:

- What screens/views are needed?
- What's the user workflow? (action → response → next screen)
- Any reference designs, inspiration, or existing patterns?
- Constraints: framework (Svelte, Ratatui, React), responsive, accessibility, theme?
- For TUI: terminal size assumptions, color support (256/truecolor), mouse support?
- For GUI: target viewport sizes, mobile support, dark mode?
- For Assets: what's needed? Required sizes/formats?

If the description is too vague, **ask clarifying questions immediately**
before proceeding to Step 2.

For **medium/large** designs (multiple screens), the Architect **SHOULD** use
`sequentialthinking` to structure the screen inventory and interaction flows
before generating mockups.

### 2. Generate Initial Mockups

Based on the design mode:

- **GUI**: use `generate_image` to create mockup images (one per screen)
- **TUI**: use ASCII/box-drawing in fenced code blocks per `design-rules.md` §3
- **CLI**: show sample command invocation + expected output
- **Assets**: use `generate_image` for icons/logos at target sizes

Label all interactive elements (buttons, inputs, hotkeys, clickable areas).
Show both default state and key interaction states where applicable.

#### MCP-Enhanced Design *(when available)*

If **Narsil MCP** is available, use it to understand existing UI code:

| Tool | Purpose |
|------|---------|
| `search_code` | Find existing UI components or templates |
| `find_symbols` | Discover existing component API surface |
| `get_project_structure` | Understand where UI code lives |

### 3. Review Loop

Present mockups to the user and iterate per `design-rules.md` §4:

1. Present mockup(s) for review
2. User reviews → gives feedback or says **"Approve"**
3. If feedback: revise specific elements, keep approved parts unchanged
4. Always present **before/after** comparison when revising
5. Repeat until all screens/assets are approved

> [!TIP]
> "Approve" is per-screen, not per-session. You can approve screen 1
> while still iterating on screen 2.

> [!CAUTION]
> If iteration count exceeds 3 on a single screen, pause and summarize
> the pattern of disagreement. Ask focused questions to converge.

### 4. Produce Design Spec

Once all screens/assets are approved:

1. Create `design/` folder with `mockups/` and `assets/` subdirectories
2. Save approved mockup images to `design/mockups/`
3. Save approved asset images to `design/assets/`
4. Fill `design/design-spec.md` per `design-rules.md` §2

Include:
- Screen inventory with relative path references to mockups
- Component inventory (buttons, inputs, panels, modals, hotkeys)
- Interaction flows (user action → system response → next screen)
- Responsive/resize behavior (GUI/TUI only)
- Asset inventory (if applicable)
- Version history

For **revision mode**: bump the version, mark revised screens with `[REVISED]`,
add entry to version history per `design-rules.md` §5.

Present the complete Design Spec to the user for final review.

### 5. Handoff

End with:

> 🛑 **Design Complete.**
> Please review the Design Spec above. You can:
> - **Revise** specific screens or assets
> - **Add** screens or interactions not yet captured
>
> When satisfied, reply with **"Plan"** to proceed to `/plan-making`.

**Do NOT proceed to planning until the user explicitly approves the Design Spec.**

## Rules

1. **No code edits** — this is a design-only workflow.
2. **No planning** — do not produce implementation steps or blueprints.
3. **Always pause** — the user must explicitly say "Plan" to move forward.
4. **Use `generate_image`** for GUI/Assets, code blocks for TUI/CLI.
5. **Always present before/after** when revising mockups.
6. **Per-screen approval** — "Approve" applies to individual screens, not the entire spec.
7. **Re-entry is scoped** — revision mode targets specific screens, not full redesign (per §5).
8. `/design` can be re-entered from `/plan-making`, `/audit`, or `/issue` per `design-rules.md` §5.
