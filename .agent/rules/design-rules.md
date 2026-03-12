# Design Rules

> Loaded by `/design` workflow. Defines design modes, spec format, mockup conventions, review loop, and re-entry protocol.

## 1. Design Modes

| Mode | Tool | Mockup Format |
|------|------|---------------|
| **GUI** (web, desktop) | `generate_image` | PNG/WebP mockup images |
| **TUI** (terminal UI) | Code blocks | ASCII/box-drawing layout in `markdown` or `text` fenced blocks |
| **CLI** (command output) | Code blocks | Sample terminal output showing command → result |
| **Assets** (icons, branding) | `generate_image` | Icon/logo images at required sizes |

### Mode-Specific Guidelines

**GUI**: Generate one mockup per screen using `generate_image`. Show both default and key interaction states (hover, selected, error). Include dark/light variants if applicable.

**TUI**: Use box-drawing characters (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`). Show focus states with highlight markers (`[>` or `*`). Note terminal size assumptions and color support.

**CLI**: Show full command invocation with expected output. Include help text, error output, and success output as separate examples.

**Assets**: Generate at target sizes. Common requirements:
- Favicon: 16×16, 32×32, 48×48
- App icon: 192×192, 512×512
- Logo: horizontal and square variants
- Splash screen: target device resolution

## 2. Design Spec Format

`design-spec.md` lives in a `design/` folder at the project root — a living, versioned document:

```
project-root/
├── design/
│   ├── design-spec.md        ← living document (like architecture.md)
│   ├── mockups/               ← approved screen mockups
│   │   ├── screen-home.png
│   │   └── screen-settings.png
│   └── assets/                ← icons, logos, branding
│       ├── favicon.png
│       └── app-icon.png
```

Template:

```markdown
## 🎨 Design Spec

| Field | Value |
|-------|-------|
| **Project** | [name] |
| **Mode** | GUI / TUI / CLI / Assets |
| **Screens** | N |
| **Version** | 1 |
| **Last Updated** | [date] |

### Screen Inventory
| # | Screen Name | Description | Mockup |
|---|------------|-------------|--------|
| 1 | [name] | [purpose] | ![name](mockups/screen-name.png) |

### Component Inventory
- [buttons, inputs, panels, modals, hotkeys, etc.]

### Interaction Flows
- [User action] → [System response] → [Next screen]

### Responsive / Resize Behavior *(GUI/TUI only)*
- [How the layout adapts to different sizes]

### Asset Inventory *(if applicable)*
| Asset | File | Sizes |
|-------|------|-------|
| Favicon | `assets/favicon.png` | 16, 32, 48 |
| App Icon | `assets/app-icon.png` | 192, 512 |

### Version History
| Version | Date | Changes |
|---------|------|---------|
| 1 | [date] | Initial design |
```

## 3. Mockup Conventions

- **One mockup per screen** — do not combine multiple views into one image
- **Label interactive elements** — buttons, inputs, hotkeys, clickable areas
- **Show key states** — default, hover/focus, selected, error, empty
- **Use consistent naming** — `screen-[name].png` for screens, `[name].png` for assets
- **TUI box-drawing reference:**

```
┌─────────────────────────────────┐
│  Title Bar                      │
├─────────────────────────────────┤
│  Content Area                   │
│                                 │
│  [> Selected Item]              │
│     Normal Item                 │
│     Normal Item                 │
├─────────────────────────────────┤
│  Status Bar          [q]uit     │
└─────────────────────────────────┘
```

## 4. Review Loop Protocol

- Each iteration: agent presents mockup → user reviews → feedback or **"Approve"**
- **"Approve"** is per-screen — can approve screen 1 while iterating on screen 2
- Track iteration count per screen (keep below 5 per screen)
- If >3 iterations on a single screen, summarize the pattern of disagreement and ask focused questions
- On **"Approve"**: save approved mockup to `design/mockups/`
- On feedback: revise specific elements, keep approved parts unchanged
- Always present **before/after** comparison when revising

## 5. Design Re-entry Protocol

Decision tree for UI problems found after design was approved:

```
UI doesn't look right
├─ Implementation doesn't match approved mockup?
│  └─ /issue (Type: bug) → /plan-making (fix code)
├─ User wants to change the approved design?
│  └─ /design (Revision mode) → update Design Spec → /plan-making
├─ Planning reveals design won't work technically?
│  └─ STOP plan → /design (Revision mode) → resume /plan-making
```

### Revision Mode Rules

1. Re-enter `/design` with the existing Design Spec
2. Scope: specify which screens/assets are being revised (not full redesign)
3. Mark revised screens with `[REVISED]` tag in the Screen Inventory
4. Keep approved screens unchanged
5. Same review loop applies (§4)
6. Bump `Version` in Design Spec header
7. Add entry to Version History table
