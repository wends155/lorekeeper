# Builder Execution Rules

> Loaded by `/build` workflow. Defines execution discipline, scope fencing, and self-verification for the Builder role.

## 1. Fidelity Hierarchy

When executing a plan, the Builder follows this priority stack:

| Priority | Source | Rule |
|:---:|:---|:---|
| **1 (highest)** | **Tests** | Write first, must pass. Tests ARE the contract. |
| **2** | **Interface Contracts** | Exact signatures, types, error variants — non-negotiable |
| **3** | **Plan step description** | Action description guides intent |
| **4 (lowest)** | **Code snippets in plan** | Illustrative guidance, not prescriptive |

> [!IMPORTANT]
> Tests define correctness. If the plan's code snippet suggests `if input.is_empty()` but the
> Builder uses a `match` expression instead, **that's acceptable** — as long as the test passes
> and the Interface Contract signature is exact.

If no test exists for a step, the plan's Action description becomes the primary fidelity target.
The Builder should match it closely since there is no test to validate against.

## 2. Step Execution Protocol

For each plan step `Step N: [TAG] file — function() (L##-##)`:

### 2.1 Parse

Extract from the step header:
- **Tag**: `[NEW]`, `[MODIFY]`, `[DELETE]`, or `[TEST]`
- **File path**: The exact file to touch
- **Function/struct name**: The specific target within the file
- **Line range**: Advisory — locate the actual target, don't blindly edit by line number

### 2.2 Read Before Write

**Before making any code changes**, the Builder MUST:
1. Read the target file (or the relevant section)
2. Locate the function/struct/module referenced in the step
3. Understand the current state — does it match the plan's Pre-condition?
4. If the Pre-condition does not hold → **STOP**, do not proceed
5. **Cross-reference the plan's Interface Contracts section** — if the step implements a struct, function, or field listed there, the signature, visibility, and field access patterns are Priority 2 (non-negotiable). Match them exactly or STOP.

### 2.3 Execute

Apply the change described in the step's Action field:
- For `[TEST]` steps: write the test **first** (TDD Red phase)
- For `[NEW]` steps: create the file with the specified content
- For `[MODIFY]` steps: edit only the targeted function/struct/lines
- For `[DELETE]` steps: remove the specified code or file

### 2.4 Post-Verify

After executing, run the step's Post checks:
1. Re-read the changed code to confirm it matches the Action description
2. Run the Post verification command(s) as specified
3. If Post check fails → enter Error Recovery (§8)

### 2.5 Update Progress

After each step completes:
- Mark `task.md`: `[ ]` → `[/]` when starting, `[/]` → `[x]` when Post passes
- At 🔒 CHECKPOINT markers: run `ALL`, commit via `Git-Checkpoint.ps1`

## 3. TDD Mandate

> [!CAUTION]
> Every code change — no matter how small — must have a corresponding test.
> A "minor" change to a helper function (e.g., email validation regex) can have
> cascading effects on all callers. Tests are the safety net, not the Builder's judgment.

**Rules:**
1. `[TEST]` steps are always executed **before** their corresponding `[MODIFY]`/`[NEW]` steps
2. The test must **fail** before implementation (Red) and **pass** after (Green)
3. If the plan omits a test for a code change, the Builder writes one anyway
4. Minor changes (§4) still require existing tests to pass — run `ALL` to verify

## 4. Scope Discipline

### 4.1 What You Can Touch

The Builder may **only** modify files and functions listed in the current step.

### 4.2 Minor Additions (Allowed)

These are pre-approved and do NOT trigger a STOP condition:
- `use` / `import` statements required by the step's new code
- `#[derive(...)]` macros implied by the step's code
- Fixing a typo in a comment **within the same function** being modified
- Formatting adjustments (handled by `rustfmt` / formatter anyway)

> [!NOTE]
> Minor additions must still pass existing tests. Run the Post check to verify.

### 4.3 Substantive Additions (NOT Allowed)

These require a **STOP** — the Builder must halt and escalate to the Architect:
- New structs, enums, traits, or public functions not in the plan
- Modifying code in a different file or function than the step targets
- Changing an existing function's signature beyond what the plan specifies
- "While I'm here" refactoring or cleanup
- Adding dependencies to `Cargo.toml` / `package.json` not in the plan

**The test:** *"Does this change exist to serve the current step, or is it an improvement I noticed?"*
If the latter → write a Builder Note (§7), do NOT make the code change.

### 4.4 Negative Scope Enforcement

If the plan includes a Negative Scope section (ipr.md §2):
1. Read it **before** starting any step
2. Before modifying any file, check it against the exclusion list
3. If a step's Action would require touching a Negative Scope item → **STOP**

## 5. Decision Boundaries

### 5.1 Allowed Micro-Decisions

The Builder may decide these without escalating:
- Variable names (when not specified in the plan)
- Comment wording (within the same function)
- `use` statement ordering
- Line breaks and whitespace (formatter handles this)
- Choosing between equivalent expressions (e.g., `if let` vs `match` for simple cases)

### 5.2 STOP Triggers

The Builder MUST **immediately halt** and escalate when:
- Adding new error variants not in the Interface Contract
- Changing a return type or function signature
- Adding public APIs (functions, structs, traits) not in the plan
- Modifying module structure (new files, moved code between modules)
- Discovering the plan contradicts `architecture.md`
- A step is ambiguous — the Builder cannot determine the intended action

> [!IMPORTANT]
> "Ambiguous" means the Builder would need to make a **design decision** to proceed.
> If the step requires only an **implementation decision** (how to code something),
> that's within scope. If it requires a **design decision** (what to code), STOP.

## 6. Self-Verification Loop

After writing code for any step:

1. **Re-read** the changed lines in the file
2. **Compare** against the step's Action description:
   - Does it implement the described behavior?
   - Does it match the Interface Contract signature (if specified)?
   - Does it respect the Negative Scope?
3. **Run** the Post check command(s)
4. **Only then** mark the step as complete in `task.md`

If the re-read reveals a mismatch:
- Fix the code **before** running Post checks
- Do not rationalize deviations — match the plan or STOP

> [!IMPORTANT]
> **Final Pipeline Rule (RC-1 Prevention):** Before running `git commit` at any checkpoint,
> the Builder MUST run the **full** verification pipeline:
> ```sh
> cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
> ```
> No code edits are permitted after the final pipeline pass. If a fix is needed, re-run the entire pipeline from the start.

## 7. Builder Notes

The Builder can flag observations and suggestions for the Architect using a structured section in `task.md`:

```markdown
## Builder Notes
- 💡 Step 5: `parse_config` could benefit from builder pattern (coding-standard §4.6.1)
- ⚠️ Step 7: Plan line range was L45-60, actual target was L55-75
- 💡 Step 9: `validate_email` has no edge-case tests for unicode — consider for next phase
```

**Rules:**
1. Builder Notes are **informational only** — the Builder does NOT act on its own suggestions
2. Only the Architect can promote a suggestion into a plan revision (during Reflect phase)
3. Use `💡` for improvement suggestions and `⚠️` for observations (mismatches, discoveries)
4. Each note references the step number and target for context
5. Notes are reviewed by the Architect during `/audit` Step 2a

## 8. Error Recovery

### 8.1 First Failure

When a Post check fails:
1. Diagnose the failure — read the error output
2. Fix **within the current step's scope** only
3. Re-run Post check
4. If fixed → continue to next step

### 8.2 Second Consecutive Failure (Same Step)

**STOP immediately.** Per `ipr.md §7`:
1. Commit current progress: `WIP: stopped at step N — [reason]`
2. Do NOT revert completed steps
3. Report the failure to the Architect with:
   - Step number and target
   - Error output from both attempts
   - What was tried

### 8.3 Never Fix Forward

Do NOT modify a **future** step's target to work around a current failure.
Each step is self-contained. If step 3 fails, do not edit step 5's target file
to compensate — that creates unaudited changes.

### 8.4 Regression Detection

If a step's Post check reveals that **previously passing tests now fail**:
1. **STOP** — this is a regression
2. Note which tests broke and which step likely caused it
3. The Architect decides rollback scope — the Builder does not unilaterally revert

---

> **Loaded by:** `/build` workflow
> **Compliance:** Verified during `/audit` via Plan Fidelity (audit-rules.md §4)
