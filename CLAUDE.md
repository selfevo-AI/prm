# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

PRM (个人资源与关系管理, v0.4) is a local-first, single-owner steward for personal Resources and Relations. A Rust/Axum server listens on 127.0.0.1, stores data in PostgreSQL, and serves a React 19 + Vite UI in Simplified Chinese.
- **Concept:** it implements the "three worlds" model from `Three_Worlds_Economic_Model_AI_and_Embodied_Intelligence_EN.docx`, extended with a self world. `docs/ARCHITECTURE.md` maps the paper to concrete constraints.
- **Specs:** the Chinese docs in `docs/` are the spec.
  - `docs/SECURITY.md`: auth, actors and vault rules.
  - `docs/ONTOLOGY-API.md`: classes, predicates, endpoints, candidate contract.
  - `docs/SELF-MODEL.md`: the psychological instrument catalog.
  - `docs/AGENT-INTEGRATION.md`, `docs/agent-openapi.json`: the external-agent API.
  - `docs/LOCAL-AI.md`: router and executor.

This directory is not a git repo, so search tools don't apply `.gitignore`. Scope searches to `src/ web/ tests/ e2e/ scripts/ migrations/ docs/` so they don't crawl `.tools/`, `node_modules/` and `target/`.

## Commands (Windows / PowerShell)

`cargo` and the bundled `gcc` are not on the default PATH, and env vars do not persist between tool calls. Run everything that builds or tests Rust in a single invocation that starts with the PATH line. The toolchain is `stable-x86_64-pc-windows-gnu`. `.tools/w64devkit` provides the gcc that compiles rusqlite's bundled SQLite. Use `npm.cmd` / `npx.cmd` from PowerShell.

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;D:\prm\.tools\w64devkit\bin;$env:Path"
.\scripts\Start-Postgres.ps1        # starts .tools/pgsql on 127.0.0.1:55432, sets $env:PRM_DATABASE_URL for this process
$env:PRM_DATABASE_SCHEMA = $null    # tests need a per-tempdir schema
cargo test --all-targets
cargo test --test agents_and_router model_routes   # one integration test (files: security_and_workflows, self_and_relationships, agents_and_router, prp)
cargo test --lib                                   # registry/catalog/router/gateway unit tests (still need PostgreSQL for lib::timeout_tests)
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo build --locked
```

- **Frontend:** `npm.cmd run build` runs `tsc -b` (strict, `noUnusedLocals`/`noUnusedParameters`; there is no ESLint) and then `vite build` into `dist/`. `npm.cmd run dev` proxies `/api` to 127.0.0.1:4317.
- **Run the app:** `.\scripts\Start-PRM.ps1 -Build [-ModelWorker] [-DataDirectory <scratch>]`.
  - Without `-Build` it reuses the existing binaries.
  - `-ModelWorker` generates a per-run `PRM_AGENT_TOKEN` and also starts `scripts/prm-model-worker.mjs`.
  - The default `data/` dir maps to the owner's real workspace.
- **E2E** (`e2e/workspace.spec.ts`, two serial tests; the second logs into the account the first creates):
  - Build first with `npm.cmd run build` and `cargo build`; `scripts/test-server.mjs` runs the existing `target/debug/prm.exe` on port 4318 with a fresh `test-data/e2e-*` dir.
  - Run with `npm.cmd run test:e2e`.
- **Scripts:** `scripts/prm-model-worker.mjs` is the local model executor (Ollama or an OpenAI-compatible server, loopback only). `scripts/prm-mcp.mjs` is the dependency-free MCP stdio bridge for external agents (`PRM_CONNECTION_TOKEN`).
- `.tools/postgresql.json` holds the DB credentials. Don't print or commit it.

## Architecture

### Request boundary and actors (`src/lib.rs`)
- There is one global `AppState = Arc<Mutex<Inner>>`, and handlers hold the lock for their whole duration. Sessions are in memory, and any login clears the other sessions.
- The middleware checks Host and Origin, requires `X-PRM-Request: 1` on non-GET requests, buffers bodies up to 128 KiB, and resolves an `Actor`:
  - `Owner`: `prm_session` cookie plus an Ed25519 signature on every `/api/` path except `auth/status|login|setup`.
  - `Reader`: the `X-PRM-Grant` header, only for `GET /api/shared/vault`.
  - `Executor`: bearer `PRM_AGENT_TOKEN`.
  - `External(id)`: bearer token issued by the owner, stored in `agent_credentials` as a SHA-256 hash.
  - `Public`: anything else.
- `Actor::allows` confines `Executor` to `/api/agent/*` and `External` to `/api/agent/import(s)`. **Every handler must still authorize itself** with `actor.owner()?`, `actor.executor()?` or `actor.external()?`, because `Public` reaches handlers.
- Owner routes live under `/api/...`, including the owner's agent config at `/api/agents/...` (plural). Bearer routes live under `/api/agent/...` (singular).
- Request signing has three implementations that must agree: `src/security.rs::verify`, `web/security.ts`, and the helpers in `tests/common/mod.rs`. So owner endpoints can't be called with plain curl.

### Ontology registry (`src/ontology.rs`): the single source of truth
- `CLASSES` holds a `ClassDef` per entity kind: label, REA category, worlds, default scenes, authority.
- `PREDICATES` holds a `PredicateDef` per relation: layer, `owner_settable`, `agent_proposable`, and a list of domain→range `signatures`.
- `relation_valid()`, `worlds()`, `scenes_for()` and `schema()` (sent to the UI) are derived from these. `Store::open` runs `sync_sql()` after `migrations/002_postgresql.sql` to sync `ontology_classes` and `relations_predicate_check`, so don't hand-edit those lists in SQL.
- **Adding a kind:**
  1. Add a `ClassDef`.
  2. If owners or agents may create it, add it to `operations::OWNER_CREATABLE` / `AGENT_CREATABLE` and to the `operations::normalize` match. Put the validator in the owning module (`prp.rs`, `relationships.rs`, …).
  3. Add a zh-CN entry to `labels` in `web/api.ts`.
- Only additive changes are safe for existing data: the sync re-adds the predicate CHECK, which fails if stored rows use a removed predicate.
- The three facets are independent: `worlds`, value axes, and `scenes` (the 8 life scenes; `entities.scenes` is a TEXT JSON column).

### Persistence (`src/database.rs`, `src/store.rs`)
- `database.rs` wraps the sync `postgres` client in a rusqlite-shaped API (`params!`, `$n` placeholders). Each call runs on its own OS thread.
- `002_postgresql.sql` runs on every startup, so all DDL there must be idempotent. There is no migration runner.
- Store methods:
  - `Store::entity(id)` queries by id; `entities_of(&[kinds])` filters by kind.
  - `insert` uses the class's default scenes; `insert_scoped` takes explicit scenes.
  - `update` is optimistic (`WHERE version = $n`), returns the bumped entity, and gives 409 on a stale version.
- Every mutation writes `store.audit(actor, "<area>.<verb>", target)` in the same transaction. The audit table is append-only.

### Domain modules
- `operations.rs`: generic creation.
  - `normalize(kind, props)` validates and `insert_entity` inserts plus the implied relations (supplies, stored_at, located_in, contains/orders, implements). Owner creates and approved agent candidates share both.
  - Also tasks, directive revoke, `relate` (registry `owner_settable`), export (`prm-operations-v2`), and the demo seed (E2E depends on its titles).
- `prp.rs` (personal resource planning, REA):
  - The resource item master, locations and suppliers.
  - The purchase order state machine `draft → ordered → partially_received → received → closed / cancelled`. `active` is the legacy draft status, and lines can only be added to drafts.
  - Goods receipt posts `receipt` movements linked by `receives`; payment posts a `cash_entry` linked by `pays`, capped at the outstanding amount. `post_movement` is the only way stock changes.
- `self_model.rs`: the static `INSTRUMENTS` catalog holds metadata and citations only, never item text. `/api/self/assessments` validates scores against the ranges. It also covers identity (TST) statements, self-relation `STANCES`, per-record `agent_visible`, and `/api/self/agent-preview`. Never compute composite scores.
- `relationships.rs`: person / relationship (IOS closeness, RAS satisfaction, contact rhythm) / interaction ledger. `contact_due` feeds the analyzer.
- `analysis.rs`: the unified decision layer.
  - Analyzer rules (`low_stock`, `owner_message`, `contact_due`, `warranty`) dedupe per (source, version, rule).
  - `stage_candidates` validates agent/model `Candidate`s into pending proposals. Entities are staged before relations so a relation can reference `#ref`.
  - `decide` dispatches on `proposal.action`: `create_task`, `create_entity` (re-runs `insert_entity`), or `create_relation` (a `#ref` must already be approved).
  - The source version is re-checked except for immutable `agent_import` sources.
- `agents/`:
  - `credentials`: issue/revoke external tokens.
  - `gateway`: `/api/agent/import` takes PRM items or OpenAI-style `messages`; it stores an immutable `agent_import` and dispatches it.
  - `router`: loopback-only `model_endpoint`s, priority `route_rule`s, and `principles_context` (active directives + agent-visible self entities, **never the vault**).
  - `executor`: offline rules run synchronously; model runs are `agent_run` entities that the worker claims under a 300 s lease (max 3 attempts, `contract: prm-run-v2`).
  - The server itself makes no outbound calls.

### Mental vault (思维体系)
- `principle`/`goal`/`reflection` are not entities. The whole vault is one XChaCha20-Poly1305 blob keyed by Argon2id(password).
- Mutations re-derive the key from the password and require `confirmed: true` and `expected_version`.
- Publishing creates a `directive` entity, optionally scene-scoped. Active directives are the "core principles" given to local models.
- Tests assert that vault text (`PRIVATE-CORE…`) never appears in the workspace, export, agent context or preview.

### Conventions
- Every request struct uses `#[serde(deny_unknown_fields)]`; tests rely on it to reject smuggled fields.
- Validate with `error::require`, `ontology::validate_text` / `validate_optional` / `validate_values` / `validate_scenes`. User-facing errors are in Simplified Chinese.
- Money is in integer cents (CNY) and quantities are integers. Ledgers (stock, cash, interactions, imports, audit) are append-only.
- Design rules:
  - AI and agents only propose; the owner approves.
  - No combined wealth or wellbeing score.
  - Psychological records are observations, not diagnoses.
  - Everything stays on loopback.

### Frontend (`web/`)
- `App.tsx` is the shell: auth, a 9-item nav, the global scene filter, and dialogs. Pages live in `web/pages/*` and receive a `PageContext` (`web/context.ts`) with `filter` (search + scene), `run` (mutate → refresh → toast), `onDialog`, and so on.
- The whole `GET /api/workspace` is fetched on login, after each mutation, and on navigation.
- Dialogs: `Editor.tsx` handles the generic and vault dialogs; `dialogs/PrpDialogs.tsx` and `dialogs/RelationDialogs.tsx` handle the new flows, dispatched by `dialog.type`.
- `api()` paths are relative to `/api` and every call except `/auth/status` is signed. `/shared` is the grant recipient page.
- E2E finds elements by zh-CN accessible names and labels, and checks for no horizontal overflow at 360–1440 px. Two constraints follow:
  - `labels.resource` must stay "资源", because relation selects use "资源 · …".
  - In dialogs with a password confirmation, scene/value checkboxes must come before the confirmation checkbox.
