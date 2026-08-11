# Rename `ouroboros-spiral` to `murex`

Date: 2026-08-11
Status: approved, not yet implemented

## Problem

The project is named `ouroboros-spiral`. That reads as an Ouroboros accessory
rather than a tool, which understates it: the risk register and the commitment
gate have no dependency on Ouroboros. Ouroboros executes the work; this project
decides which risk each cycle must retire and whether the next cycle earns its
cost. Nothing in that is Ouroboros-specific.

The name also blocks a direction we want to keep open - running the same
controller under Claude Code, Codex, or gjc directly, with Ouroboros demoted to
one adapter among several.

## Decision

Rename to **`murex`** - a spiral-shelled sea snail, and the source of Tyrian
purple in antiquity. The shell is the spiral; extracting the dye meant breaking
the shell, which is the same bargain this tool makes explicit: you spend a cycle
to learn something you cannot learn without spending it.

`spiral` steps down from the name to the domain vocabulary. Documentation keeps
calling the method the spiral model; the tool is `murex`.

### Scope: rebranding plus a neutral state path

Three options were considered:

| Option | Scope | Verdict |
|---|---|---|
| A | Crate and repo name only; keep `.ouroboros/` state | Rejected - leaves the tool's own data under another project's directory |
| B | Full harness independence; Ouroboros becomes one adapter | Rejected for now - more change than the rename needs, and the Ouroboros integration currently works |
| C | Rename plus a neutral state path, Ouroboros integration retained | **Chosen** |

C is the smallest change that removes the subordination without discarding
working integration. It leaves B reachable later: once state lives under
`.murex/`, adding a second harness adapter touches no existing data.

### Command namespace: unified

The plugin manifest's `namespace` becomes `murex`, so the command is
`ooo murex cycle` rather than `ooo spiral cycle`.

The alternative - tool named `murex`, namespace kept as `spiral` - was rejected.
Two names for one thing taxes every document, issue, and conversation with a
parenthetical. Discoverability, the one real argument for keeping the
self-describing `spiral`, is served by the skill instead: `SKILL.md` keeps
"spiral-model" and "risk-driven" in its `description` frontmatter, which is what
agents actually match against.

## Changes

| Target | Before | After |
|---|---|---|
| Crate, binary | `ouroboros-spiral` | `murex` |
| Library | `ouroboros_spiral` | `murex` |
| Manifest `name`, `namespace` | `spiral` | `murex` |
| Command usage strings | `ooo spiral <cmd>` | `ooo murex <cmd>` |
| Skill directory | `skills/spiral/SKILL.md` | `skills/murex/SKILL.md` |
| Skill frontmatter `name` | `spiral` | `murex` |
| State path (`STATE_PATH`) | `.ouroboros/spiral.json` | `.murex/spiral.json` |
| Repository directory | `~/workspace/ouroboros-spiral` | `~/workspace/murex` |

The state file keeps the name `spiral.json`: the directory identifies the tool,
the file identifies the artifact, which leaves room for `.murex/` to hold
something else later without a second migration.

Error and guidance strings embedded in the library (`load`, `open_cycle`,
`commit`, and the `next` hints returned by `start` and `open_cycle`) quote the
command form and must be updated in step with the namespace.

## Deliberately unchanged

- **`ouroboros.plugin.json` filename.** Dictated by the plugin contract, not by
  us. A host convention living under its own name is correct.
- **The Ouroboros manifest itself.** Option C retains the integration.
- **On-disk JSON structure.** Only the path moves; keys, types, and semantics
  are untouched.
- **No migration code.** There are zero state files in real use. Writing a
  converter for a path nobody has written to would be the project's first piece
  of debt. If a real spiral is ever stranded at `.ouroboros/spiral.json`, moving
  the file by hand is the whole migration.

## Verification

Unchanged behaviour is the acceptance criterion - the rename must be observable
only in names.

1. `cargo test` - the two existing tests pass (exposure ranking and the cycle
   gate; numeric id tie-break past ten).
2. `cargo clippy --all-targets -- -D warnings` - clean.
3. Manifest validates against the Ouroboros core plugin schema 0.1.
4. Release binary end-to-end: `start` → `risk add` → `cycle` → `commit`
   produces the same JSON as before the rename, with state at `.murex/spiral.json`.
5. `grep -ri ouroboros` over tracked files returns only the intended survivors:
   the `ouroboros.plugin.json` filename, the upstream repository URL, prose
   describing the Ouroboros integration, and this design document, which is a
   record of the rename and necessarily names the old owner.

## Sequencing

The repository directory rename must come last. The working session runs inside
`.claude/worktrees/rust-port` beneath the repository, so moving the parent
directory first would break the session's working path.

1. Rename inside the repository (code, manifest, skill, docs).
2. Run the verification set.
3. Commit on the working branch.
4. Landing the branch on `master` and removing the worktree is the maintainer's
   call, not part of this change.
5. Rename `~/workspace/ouroboros-spiral` to `~/workspace/murex`, after step 4,
   once no session is working beneath that directory.

## Out of scope

Publishing to crates.io. `murex` is available there as of 2026-08-11, which
preserves the option, but distribution stays `cargo install --path .` until
there is a reason to publish.
