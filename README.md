# Dreamcatcher

Dreamcatcher is the compiler of the Dream suite. It discovers `.dream` files,
builds a Dream IR graph, runs deterministic analysis passes, and emits
`.deliver` execution contracts that Dreamboat acts upon.

```
.dream ──► Dreamcatcher ──► .deliver ──► Dreamboat ──► execution
```

> Dreams are possibilities. Delivers are decisions. Dreamboat is action.

## Suite topology

| Component     | Role                                                    | Artifact     |
|---------------|---------------------------------------------------------|--------------|
| Dreamsequence | Observes reality and generates/refines dreams          | `.dream`     |
| **Dreamcatcher** | Discovers, compiles, clusters, reconciles, reasons | `.deliver`   |
| Deliver       | Verifies deliverables deterministically                | deliver.toml |
| Dreamboat     | Determines how to execute and performs the work        | execution    |

## Pipeline

```
.dreams → discovery/index → Dream IR graph
        → clustering & merging
        → conflict analysis
        → dependency analysis
        → reasoning/synthesis
        → prioritisation
        → .deliver contracts (YAML + JSON sidecar) + manifest.json DAG
        → Dreamboat
```

All passes are deterministic (no LLM). Where interpretation is required the
contract carries a `required_human_decisions` list instead of guessing, and
every contract has a `provenance` block tracing it back to the `.dream`
evidence (id, path, sha256) that caused it.

## The `.dream` format

Human-writable TOML:

```toml
id = "dream-001"
title = "Kaptaind forward compatibility"

[intent]
statement = "old .kaptaind files should keep working"
kind = "proposal"           # opportunity|desire|observation|proposal|concern|change

[subject]
name = "kaptaind"
tags = ["config", "schema"]

[evidence]
items = ["migration broke in v0.3"]

[meta]                       # everything here is optional
priority_hint = "high"       # low|medium|high|critical
effort_hint = "medium"       # small|medium|large
risk_hint = "medium"         # low|medium|high
depends_on = []
conflicts_with = []
supersedes = []
requires_capabilities = ["schema detection"]
open_questions = ["which schema registry?"]
```

If `id` is omitted, one is derived from the filename and content hash.
Run `dreamcatcher schema` for the JSON Schema.

## Usage

```
dreamcatcher                      # compile everything under ./
dreamcatcher --root ~/dreams -o deliverables
dreamcatcher validate             # parse-check only
dreamcatcher ir --emit ir.json    # dump the IR graph
dreamcatcher --dry-run --format json
dreamcatcher -v                   # info-level telemetry; -vv for debug
dreamcatcher init                 # scaffold dreamcatcher.toml
```

## Output & telemetry

Text output is emoji-led and (on an interactive terminal) colourised with
an animated moon-phase spinner during long phases:

```
🌙 dreamcatcher 0.1.x — compiling the dreamspace

📜 3 dreams   🕸️ 1 clusters   ⚔️ 2 conflicts   🎯 2 deliverables

🎯 deliverables (priority order)
┌──────────────────────┬──────────────────────────┬──────────┬──────┬──────┬─────────────────┐
│ deliverable          │ slug                     │ priority │ ⚡   │ 📜   │ status          │
├──────────────────────┼──────────────────────────┼──────────┼──────┼──────┼─────────────────┤
│ deliver-conflict-001 │ kaptaind-evolution-2-dr… │ high     │ 42   │ 2    │ 🙋 needs decision │
└──────────────────────┴──────────────────────────┴──────────┴──────┴──────┴─────────────────┘

🔍 scanned 12034 files across /home/you/dreams
⏱️ discovery 820ms · analysis 3ms · emit 4ms · total 827ms
```

- **No more silent walking.** Discovery skips all hidden directories plus
  `target/` and `node_modules/` (an explicitly passed `--root` is always
  entered), reports every 1000 files scanned on the spinner, and the final
  report carries files-scanned and per-phase timings. Earlier builds
  appeared to hang because they walked `.rustup`-scale trees silently.
- **Verbosity**: warnings by default; `-v` adds per-phase `tracing` info
  (including every file written); `-vv` adds debug detail. `RUST_LOG`
  overrides.
- **Machine-friendly**: `--format json` emits pure JSON on stdout with the
  same telemetry (`timings`, `files_scanned`, `roots`) while phase lines
  stay on stderr. Colours and the spinner disable automatically for pipes
  and honour `NO_COLOR`.

Outputs in the `--out` directory:

- `<slug>.deliver` — the YAML execution contract (scope, objective,
  required_capabilities, constraints, acceptance, priority, dependencies,
  execution: Dreamboat, provenance)
- `<slug>.deliver.json` — a dreamboat-compatible `JourneyPayload` sidecar
  (unsigned in v0.1)
- `manifest.json` — the deliver DAG in priority order, with scores and
  human-decision flags

## Compilation semantics

- Dreams sharing a subject or exceeding the similarity threshold converge
  into one higher-order deliverable (threshold configurable in
  `dreamcatcher.toml`).
- Declared or detected conflicts are never merged: each pair becomes a
  `needs_human_decision` deliverable.
- Missing `depends_on` references and dependency cycles are hard errors.
- Open questions become explicit `required_human_decisions` on the contract.

## Development

```
cargo test          # unit + integration tests
deliver --spec deliver.toml   # self-check (files, cargo check, cargo test)
```

Monitored by kaptaind; versioned via `VERSION` (kept in sync with
Cargo.toml and man/dreamcatcher.1).

## Out of scope for v0.1

LLM-assisted reasoning, HMAC signing of JSON sidecars, dreamsequence
integration, daemon/watch mode.
