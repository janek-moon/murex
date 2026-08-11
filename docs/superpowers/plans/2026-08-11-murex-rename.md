# murex Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename `ouroboros-spiral` to `murex` and move its state out of the host's directory, so the tool stops reading as an Ouroboros accessory.

**Architecture:** Pure rename. Four surfaces change independently — the Rust crate, the plugin manifest, the skill, and the README — and each is verified on its own. Behaviour is the invariant: the same commands must produce the same JSON, only under new names and at `.murex/spiral.json`.

**Tech Stack:** Rust 2021 (chrono, clap, serde, serde_json; tempfile for tests), Ouroboros UserLevel plugin manifest schema 0.1.

**Spec:** `docs/superpowers/specs/2026-08-11-rename-design.md`

## Global Constraints

- Behaviour must be observable only in names. Same inputs produce the same JSON output.
- On-disk JSON structure is untouched: keys, types, and semantics stay identical. Only the path moves.
- No migration code from `.ouroboros/spiral.json`. Zero state files exist in real use.
- The `ouroboros.plugin.json` **filename** does not change — it is dictated by the plugin contract.
- The Ouroboros integration is retained. This is a rename, not a decoupling.
- The manifest must validate against Ouroboros core plugin schema `0.1`.
- `skills/murex/SKILL.md` frontmatter `description` must keep the words `spiral-model` and `risk-driven` — that field is the agent's discovery surface, and it is what replaces the self-describing `spiral` namespace.
- `spiral` survives as domain vocabulary in prose and in the state filename. It is removed only from names the user types and names the toolchain resolves.
- Do not publish to crates.io.
- The repository directory rename happens after the branch lands, and is not part of Tasks 1-4.

---

### Task 1: Rename the crate and neutralize the state path

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Test: `tests/spiral.rs`

**Interfaces:**
- Consumes: nothing — this is the first task.
- Produces: crate `murex`, providing both the library `murex` and the binary `murex`. Public API is unchanged in every signature: `start`, `add_risk`, `close_risk`, `list_risks`, `open_cycle`, `commit`, `stop`, `status`, `load`, `save`, `ranked_open_risks`, `exposure`, plus the types `Risk`, `Cycle`, `Spiral`, `SpiralError`, `Result<T>` and the constants `STATE_PATH`, `DECISIONS`, `OPEN_STATES`. Only `STATE_PATH`'s **value** changes, to `".murex/spiral.json"`. Tasks 2-4 depend on the binary being named `murex` and on that constant.

- [ ] **Step 1: Write the failing test**

In `tests/spiral.rs`, change the import on line 5 from `use ouroboros_spiral as sp;` to:

```rust
use murex as sp;
```

Then append this test to the end of the file:

```rust
#[test]
fn state_lands_under_the_tool_directory() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();
    sp::start(root, "path check", vec![], vec![]).expect("start");
    assert_eq!(sp::STATE_PATH, ".murex/spiral.json");
    assert!(root.join(".murex/spiral.json").exists());
    // The host's directory is no longer ours to write into.
    assert!(!root.join(".ouroboros").exists());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test`
Expected: FAIL to compile, with `error[E0432]: unresolved import` or `use of undeclared crate or module 'murex'`. The crate is still named `ouroboros_spiral`.

- [ ] **Step 3: Rename the package in `Cargo.toml`**

Replace the whole file with:

```toml
[package]
name = "murex"
version = "0.1.0"
edition = "2021"
description = "Boehm spiral-model cycle controller for Ouroboros"
license = "MIT"

[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

The `[lib]` and `[bin]` sections are deleted on purpose. They existed only to give the targets names that differed from the package name. Now that all three are `murex`, Cargo derives the library and the binary from `package.name` plus the presence of `src/lib.rs` and `src/main.rs`. Keeping them would be two more places to edit at the next rename.

- [ ] **Step 4: Update `src/lib.rs`**

One constant and seven operator-facing strings. Each `ooo spiral` becomes `ooo murex`; the state path loses the host's directory.

| Location | Before | After |
|---|---|---|
| `STATE_PATH` const | `".ouroboros/spiral.json"` | `".murex/spiral.json"` |
| `load`, not-found error | ``run `ooo spiral start \"<objective>\"` first`` | ``run `ooo murex start \"<objective>\"` first`` |
| `start`, already-exists error | ``see `ooo spiral status` `` | ``see `ooo murex status` `` |
| `start`, returned `next` | `ooo spiral risk add "<risk>" ...` | `ooo murex risk add "<risk>" ...` |
| `open_cycle`, pending-cycle error | ``close it with `ooo spiral commit --decision ...` `` | ``close it with `ooo murex commit --decision ...` `` |
| `open_cycle`, no-risks error | ``with `ooo spiral risk add`, or ... `ooo spiral stop` `` | ``with `ooo murex risk add`, or ... `ooo murex stop` `` |
| `open_cycle`, returned `next` | ``then `ooo spiral commit ... --resolve {top_id}` `` | ``then `ooo murex commit ... --resolve {top_id}` `` |
| `commit`, no-open-cycle error | ``start one with `ooo spiral cycle` `` | ``start one with `ooo murex cycle` `` |

Leave the module doc comment's references to `ooo auto`, `ooo run`, and `ooo evolve` alone — those are Ouroboros core commands, not ours. Leave the word "spiral" in prose and in the `spiral.json` filename.

- [ ] **Step 5: Update `src/main.rs`**

Three changes:

```rust
// 1. the import
use murex as spiral;
```

```rust
// 2. the clap program name
#[command(name = "ooo murex", about = "Risk-driven spiral-model cycles.")]
```

```rust
    /// Repository root holding .murex/spiral.json.
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
```

- [ ] **Step 6: Verify no stale names remain in the crate**

Run: `grep -rn "ooo spiral\|ouroboros_spiral\|\.ouroboros/" src/ tests/ Cargo.toml`
Expected: no output. If anything matches, fix it and re-run before continuing.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS — three tests in `tests/spiral.rs` (`spiral_drives_cycles_by_risk_exposure`, `risk_ids_rank_numerically_past_ten`, `state_lands_under_the_tool_directory`).

- [ ] **Step 8: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: `Finished` with no warnings.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/main.rs tests/spiral.rs
git commit -m "Rename crate to murex and move state to .murex/

The risk register and the commitment gate never depended on Ouroboros, so
the tool stops living under the host's name and the host's directory.

Behaviour and the on-disk JSON structure are unchanged; only STATE_PATH and
the command strings move."
```

---

### Task 2: Rename the plugin manifest surface

**Files:**
- Modify: `ouroboros.plugin.json`

**Interfaces:**
- Consumes: the binary `murex` from Task 1, which becomes the manifest's `entrypoint.command`.
- Produces: a manifest declaring `name: "murex"` and namespace `murex` across all six commands, so the invocation is `ooo murex <cmd>`. Tasks 3 and 4 document that invocation.

- [ ] **Step 1: Confirm the manifest still carries the old names**

Run: `grep -c '"namespace": "spiral"' ouroboros.plugin.json`
Expected: `6` — one per command. This is the baseline the next step removes.

- [ ] **Step 2: Edit the manifest**

The filename stays `ouroboros.plugin.json`. Inside it, change:

1. Top-level `"name": "spiral"` → `"name": "murex"`.
2. All six `"namespace": "spiral"` → `"namespace": "murex"`.
3. Every `usage` string, replacing `ooo spiral` with `ooo murex`:

```
ooo murex start "<objective>" [--constraint <text>] [--alternative <text>]
ooo murex risk add "<risk>" --probability 0.7 --impact 0.9 | ooo murex risk list | ooo murex risk close <id>
ooo murex cycle [--objective <text>]
ooo murex commit --decision <continue|pivot|stop> [--cost <n>] [--resolve <risk-id>]
ooo murex stop [--reason <text>]
ooo murex status
```

4. `"entrypoint"`:

```json
  "entrypoint": {
    "type": "command",
    "command": "murex"
  }
```

5. The `state` capability `reason`, which names the file:

```json
    {
      "name": "state",
      "access": "write",
      "reason": "Persist the risk register and cycle history in .murex/spiral.json."
    }
```

Leave the top-level `description` as it is — it says "Boehm spiral-model cycle controller", which is domain vocabulary, not a name.

- [ ] **Step 3: Validate against the Ouroboros core schema**

```bash
python3 - <<'PY'
import json, urllib.request
url = ("https://raw.githubusercontent.com/Q00/ouroboros/main/"
       "src/ouroboros/plugin/schemas/0.1/plugin.schema.json")
schema = json.load(urllib.request.urlopen(url))
manifest = json.load(open("ouroboros.plugin.json"))
import jsonschema
jsonschema.validate(manifest, schema)
print("VALID; name =", manifest["name"],
      "| namespace =", manifest["commands"][0]["namespace"],
      "| entrypoint =", manifest["entrypoint"]["command"])
PY
```

Expected: `VALID; name = murex | namespace = murex | entrypoint = murex`

- [ ] **Step 4: Have the real loader read it**

`discover` inspects a manifest without writing a lockfile or a trust store, so it is safe to run against the live install.

Run: `uvx --from ouroboros-ai ouroboros plugin discover .`
Expected: it reports the plugin as `murex` with six commands under the `murex` namespace, and exits 0.

If it exits non-zero, read the error before changing anything — a schema-valid manifest that the loader rejects means the loader enforces something the schema does not, and that constraint belongs in this plan, not in a guess.

- [ ] **Step 5: Commit**

```bash
git add ouroboros.plugin.json
git commit -m "Rename the plugin manifest surface to murex

Namespace unifies to \`ooo murex <cmd>\`. Carrying two names for one tool
taxes every document and conversation with a parenthetical; discoverability
moves to the skill description, which is what agents match on.

The ouroboros.plugin.json filename stays - it is the host's contract."
```

---

### Task 3: Rename the skill

**Files:**
- Move: `skills/spiral/SKILL.md` → `skills/murex/SKILL.md`
- Modify: `skills/murex/SKILL.md`

**Interfaces:**
- Consumes: the `ooo murex <cmd>` invocation established in Task 2.
- Produces: the agent-facing surface. This is what Claude Code and Codex match against and read; nothing else depends on it.

- [ ] **Step 1: Move the directory under version control**

```bash
git mv skills/spiral skills/murex
```

- [ ] **Step 2: Update the frontmatter and the title**

The `name` must match the manifest namespace. The `description` must not lose `spiral-model` or `risk-driven` — with the namespace no longer self-describing, this field is the entire discovery surface.

```markdown
---
name: murex
description: "Run risk-driven spiral-model cycles: register risks, de-risk the largest one per cycle, gate on a commitment review"
---

# ooo murex - Risk-Driven Spiral Cycles
```

- [ ] **Step 3: Update every command example in the body**

Replace `ooo spiral` with `ooo murex` throughout, and the state path in the Notes section:

```markdown
- State lives in `.murex/spiral.json`; commit it to share the register.
```

Leave every other use of the word "spiral" — "the spiral model", "a spiral with no risks is just a slower waterfall", "the spiral converges" — untouched. Those name the method, not the tool.

- [ ] **Step 4: Verify no stale invocation remains**

Run: `grep -rn "ooo spiral\|\.ouroboros/" skills/`
Expected: no output.

Run: `grep -c "spiral-model\|risk-driven" skills/murex/SKILL.md`
Expected: at least `1` — the constraint on the description field is still met.

- [ ] **Step 5: Commit**

```bash
git add skills/
git commit -m "Rename the skill to murex

The description keeps 'spiral-model' and 'risk-driven': with the namespace
no longer self-describing, that field is the whole discovery surface."
```

---

### Task 4: Update the README and correct the plugin-install claim

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: the binary name from Task 1, the namespace from Task 2, the skill path from Task 3.
- Produces: nothing other tasks read.

The README currently states that `ouroboros plugin add` has not shipped in a release. **That is false** — the released `ouroboros-ai` CLI has a full `plugin` command group (`add`, `install`, `trust`, `discover`, `list`, `inspect`, `disable`, `remove`). The claim came from a truncated reading of `ouroboros --help`. Removing it is part of this task, not a separate concern, because the same paragraph is being rewritten for the rename.

- [ ] **Step 1: Replace `README.md`**

```markdown
# murex

A Boehm spiral-model cycle controller, packaged as an [Ouroboros](https://github.com/Q00/ouroboros)
UserLevel plugin. Rust, single binary.

Named for the spiral-shelled sea snail that Tyrian purple was extracted from -
you had to break the shell to get the dye, which is the bargain this tool makes
explicit: you spend a cycle to learn what you cannot learn without spending it.

## Why

Ouroboros already drives execution and iteration. Its `evolve` loop is
**quality-driven**: it regenerates until an evaluation gate passes. The spiral
model is **risk-driven**: each cycle exists to retire the largest risk, and a
commitment review between cycles decides whether the next one earns its cost.

That risk quadrant is the one thing Ouroboros does not have, so it is the only
thing this plugin adds. It holds the register, ranks by exposure, picks what
each cycle must de-risk, and gates the commitment review. It never executes
work - `ooo murex cycle` emits a spike brief you hand to `ooo auto` or
`ooo run`.

## Build and install

```bash
cargo install --path .              # puts `murex` on PATH
ouroboros plugin discover .         # inspect the manifest, writes nothing
ouroboros plugin add . --plugin murex
```

The manifest's entrypoint is the bare binary name, so the plugin resolves it
from PATH. The binary also runs standalone, with identical argv and JSON output:

```bash
murex --root <target-repo> status
```

## Use

```bash
ooo murex start "ship realtime collaborative editing"
ooo murex risk add "CRDT memory may exceed the 2GB box" --probability 0.6 --impact 0.9
ooo murex cycle                     # -> spike brief for the top-exposure risk
# ... execute the brief through ooo auto / ooo run ...
ooo murex commit --decision continue --cost 1.5 --resolve R1 --evidence "380MB RSS"
ooo murex status                    # radius + remaining exposure
```

`skills/murex/SKILL.md` is the agent-facing surface: it is what teaches Claude
Code, Codex, and the other runtimes when to reach for a spiral and how to drive
one. The full walkthrough lives there.

## Layout

| Path                     | Role                                          |
|--------------------------|-----------------------------------------------|
| `ouroboros.plugin.json`  | Plugin manifest (schema 0.1)                  |
| `src/lib.rs`             | Controller logic - register, ranking, gate    |
| `src/main.rs`            | CLI entrypoint; argv in, JSON out             |
| `skills/murex/SKILL.md`  | In-agent surface for Claude Code / Codex      |
| `tests/spiral.rs`        | Self-check: `cargo test`                      |

The manifest keeps the filename `ouroboros.plugin.json` because the plugin
contract dictates it. State is written to `.murex/spiral.json` in the target
repository, as plain JSON, so the register stays readable and diffable in review.

## Scope

Risk scoring is human judgement, entered through `risk add`. The plugin does
not call a model to guess probabilities - it is the deterministic bookkeeping
and the gate, so that whatever the agent claims about a risk stays auditable
against the evidence recorded when the risk was closed.

MIT.
```

- [ ] **Step 2: Verify the install path the README now promises**

Install into a sandbox so the live `~/.ouroboros` is untouched:

```bash
SANDBOX=$(mktemp -d)
uvx --from ouroboros-ai ouroboros plugin add . --plugin murex \
  --plugin-home-root "$SANDBOX/plugins" \
  --lockfile "$SANDBOX/lock.json" \
  --trust-root "$SANDBOX/trust"
echo "exit=$?"
```

Expected: exit 0, and `$SANDBOX/lock.json` contains an entry named `murex`.

If it fails because the repository has no plugin catalog (`--plugin` is
documented as "name of a plugin in the repo catalog"), try the single-manifest
form instead:

```bash
uvx --from ouroboros-ai ouroboros plugin install . \
  --plugin-home-root "$SANDBOX/plugins" \
  --lockfile "$SANDBOX/lock.json" \
  --trust-root "$SANDBOX/trust"
```

Whichever of the two succeeds is the command the README must show. Edit the
README's install block to match the one that actually worked, and record the
observed failure of the other in the commit message. Do not leave a command in
the README that has not been run.

- [ ] **Step 3: Final audit across all tracked files**

Run: `grep -rn "ooo spiral\|ouroboros_spiral\|ouroboros-spiral\|\.ouroboros/" $(git ls-files)`

Expected survivors, and nothing else:
- `docs/superpowers/specs/2026-08-11-rename-design.md` and `docs/superpowers/plans/2026-08-11-murex-rename.md` — records of the rename, which necessarily name the old owner.

Any hit in `src/`, `tests/`, `skills/`, `README.md`, `Cargo.toml`, or `ouroboros.plugin.json` is a miss from an earlier task. Fix it here.

Then confirm the intended `ouroboros` references are still present:

Run: `grep -rn "ouroboros" README.md ouroboros.plugin.json | grep -v "^Binary"`
Expected: the upstream repository URL, the `ouroboros plugin` install commands, and the `ouroboros.plugin.json` filename in the layout table. All three are correct.

- [ ] **Step 4: Re-run the full verification set**

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: three tests pass, clippy clean. Behaviour is the invariant; if either fails, the rename changed something it should not have.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "Rewrite the README for murex and drop a false claim

The README said \`ouroboros plugin add\` had not shipped in a release. It
has - the released CLI carries a full plugin command group. The claim came
from a truncated reading of \`ouroboros --help\`.

The documented install command is the one that was actually run."
```

---

### Task 5: Rename the repository directory (post-merge, maintainer)

**Not executed as part of this plan.** The working session runs inside
`.claude/worktrees/rust-port` beneath the repository, so moving the parent
directory while any session is working under it breaks that session's path.

After the branch lands on `master` and the worktree is removed, the rename is:

```bash
git -C ~/workspace/ouroboros-spiral worktree list   # expect only the main checkout
mv ~/workspace/ouroboros-spiral ~/workspace/murex
```

Nothing inside the repository refers to its own directory name, so no file
changes accompany this step.

---

## Self-Review

**Spec coverage.** Every row of the spec's Changes table maps to a task: crate,
binary, and library names → Task 1, Step 3; manifest `name`, `namespace`, and
usage strings → Task 2, Step 2; skill directory and frontmatter → Task 3, Steps
1-2; `STATE_PATH` → Task 1, Step 4; repository directory → Task 5. The spec's
"Error and guidance strings ... must be updated in step with the namespace" is
Task 1, Step 4's table, which enumerates all seven. The spec's five verification
items map to Task 1 Steps 7-8 (tests, clippy), Task 2 Step 3 (schema), Task 4
Step 2 (end-to-end install), and Task 4 Step 3 (the grep audit, including the
design document as an expected survivor). The spec's "no migration code"
constraint appears in Global Constraints and is enforced by Task 1 Step 1's
assertion that `.ouroboros` is never created.

**Placeholder scan.** No TBD, TODO, or "handle errors appropriately". The one
conditional branch — Task 4 Step 2's fallback from `plugin add` to
`plugin install` — states both commands in full and the rule for choosing
between them, rather than deferring the decision.

**Type consistency.** `STATE_PATH` is the same identifier in Task 1's interface
block, Step 4's table, and Step 1's assertion. The binary name `murex` is
consistent across Task 1's `Cargo.toml`, Task 2's `entrypoint.command`, and
Task 4's README. The namespace `murex` is consistent between Task 2's manifest
and Task 3's skill frontmatter `name`, which the plan requires to match.
