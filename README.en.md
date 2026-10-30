# PRM · Personal Resource & Relation Management

[简体中文](README.md) | English

A local-first application built on the ideas of `Three_Worlds_Economic_Model_AI_and_Embodied_Intelligence_EN.docx`. Rust backend + PostgreSQL + a React UI in Simplified Chinese and English, offline by default.

PRM is a steward and agent for personal information, Resources and Relations, built on an ontology. R stands for both Resource and Relation: the self ontology holds the body and mind as available resources, and also the owner's relations to the outside world — and to the self. Every entity is organized along three mutually independent facets: **worlds** (self, mental, digital, physical), **value axes** (need, desire, existence), and **life scenes** (self, emotion, life, work, health, learning, finance, social). Upper-level categories follow the REA (Resource–Event–Agent) ontology, extended with commitments, relations and observations.

The current version is 0.5:

- The digital world gains connections to mainstream AI agents (Grok, Claude, ChatGPT, Gemini, Copilot, Muse, DeepSeek, Dify, n8n, LangChain, Cursor, Cline, …): picking a platform and issuing a token registers the agent as a digital-world entity and opens **a relation between you and the AI agent in the life scenes you choose** — connecting an agent *is* establishing that relation. Imports arrive through HTTP / OpenAPI or the MCP tool bridge, and a copy-ready connection snippet is provided.
- The UI is available in Simplified Chinese and English (switched in the top bar, Chinese by default), and the project documentation has an English edition.
- The self ontology gains a catalog of mainstream psychological instruments, “I am …” identity statements and stances toward the self.
- Relations manage relationships with people, relations with AI agents, and an interaction ledger.
- The digital world provides import interfaces for mainstream AI agents (HTTP / OpenAPI, MCP). Imports are routed to offline rules or a local model; the local model works from the core principles you published and the self records you made visible. Every result enters the decision center as a proposal you review before it joins the ontology.
- The physical world follows PRP (Personal Resource Planning): items, storage locations, suppliers, purchase orders, goods receipt and payments.

## Running

In PowerShell:

```powershell
cd D:\prm
.\scripts\Start-PRM.ps1 -Build
```

The default address is `http://127.0.0.1:4317`; if the port is taken the script picks the next one and prints the actual address and process id. On first launch you set your own account password; the mental vault starts empty. Never commit a real password to code, chat logs or environment variables.

Add `-ModelWorker` to also run a local model: the script generates an executor token for this run and starts `scripts/prm-model-worker.mjs` (see [Local models and routing](docs/LOCAL-AI.md), in Chinese).

This machine already has the Rust GNU toolchain, and `.tools/w64devkit` provides the C compiler. A local PostgreSQL 17.9 lives in `.tools/pgsql`; the start script launches it on 127.0.0.1:55432 with a non-superuser role `prm`. Random database credentials live in `.tools/postgresql.json`, readable only by the current user — never commit that file. With an existing database, set `PRM_DATABASE_URL` and the script will use it. Database connections only accept explicit loopback addresses.

On another machine, install PostgreSQL 17+ (or unpack the official binaries into `.tools/pgsql`), create a non-superuser application role and its own database, then set `PRM_DATABASE_URL=postgresql://<user>:<password>@127.0.0.1:<port>/<database>`. Setting `PRM_DATABASE_SCHEMA=prm_main` explicitly makes migration and recovery easier. You also need Node.js 22+, a stable Rust toolchain and a C compiler for your platform. On Windows we recommend Rust MSVC with Visual Studio C++ Build Tools; GNU + MinGW also works. `.cargo/config.toml` points GNU builds at Rust's bundled runtime and does not affect MSVC/Linux/macOS.

## What is implemented

| Module | Workflows |
| --- | --- |
| Workbench | Tasks, proposals, net cash, resource and contact reminders; life-scene overview that filters every view |
| Self ontology | Body metrics (sleep, cardiovascular, body composition, activity, subjective state); a catalog of psychological instruments (RSES, SCCS, CSES, GSES, SCS-SF, BPNSFS, SWLS, WHO-5, Ryff, I-PANAS-SF, PSS-10, BRS, BFI-2-S, MAAS, WLEIS, ECR-R) with score ranges and trends; “I am …” identity statements; stances toward the self; per-record agent visibility and a core-principles preview |
| Relations | People and relationships (type, IOS closeness, RAS satisfaction, contact rhythm), relations with AI agents (platform, relation type, scenes); an interaction ledger; contact proposals when the rhythm is exceeded |
| Mental vault | Principles, goals, reflections; owner-signed, password-confirmed edits; read-only grants per item, public key and expiry, with revocation; version history; publishing directives (optionally scene-scoped) as the local model's core principles |
| Digital space | Multi-conversation messages, offline-assistant replies, knowledge notes, directives; agent connections: platform presets (Grok, Claude, ChatGPT, Gemini, Copilot, Muse, DeepSeek, Dify, n8n, LangChain, Cursor, Cline, …), relation-on-connect (relation type and scenes), one-time tokens with copy-ready snippets, import records, local models, routing rules, routing runs |
| Physical resources | Item master data (consumable/asset, SKU, reorder point, reorder quantity, lead time, warranty), storage locations, suppliers; purchase orders draft → ordered → received → paid → closed; receipts, issues, stocktake adjustments; append-only stock and cash ledgers |
| Decisions | One queue of proposals: suggest a task, add an entity, create a relation; shows source, executor and referenced core principles; accept or reject after editing the title or linking a directive |
| Ontology | Classes across the four worlds, REA categories, value axes and life scenes; relation constraints grouped into the self, external, PRP, digital and provenance layers; entity browsing and manual relation creation |
| Activity | The last 200 operations with their actor (owner, offline assistant, local executor, external agent, read-only recipient) and a business JSON export |

“Add business demo data” is an explicit owner action: it adds business samples only and never creates mental-vault entries or self records. Tests run in isolated temporary directories, unrelated to `data/`.

## Connecting AI agents

In Digital space · Agent connections, pick a platform (Grok, Claude, ChatGPT, Muse, …) and issue a token. Connecting does three things at once: it registers the agent as a digital-world entity, opens a relation between you and it (choose a relation type and life scenes; manage and log interactions in Relations), and generates a one-time token that is shown only once.

- HTTP / OpenAPI: put the token into any agent that can make local HTTP requests (Dify, n8n, LangChain, custom bots, …); the interface is in [agent-openapi.json](docs/agent-openapi.json).
- MCP tool bridge: mount `scripts/prm-mcp.mjs` in Claude Desktop, Cursor, Cline and similar clients.
- A token can only import information that awaits review. It cannot read any data; imports land in the decision center for the owner to review.

See [Agent integration](docs/AGENT-INTEGRATION.en.md) (also in Chinese: [docs/AGENT-INTEGRATION.md](docs/AGENT-INTEGRATION.md)).

## Design basis

The source paper treats the three worlds as intersecting dimensions, and “need / desire / existence” as independent value axes. The software keeps that, and adds life scenes that are independent of both: a task can belong to the digital and physical worlds at once, and any entity can carry several value axes and scenes. The mental vault's security level is independent of world and scene tags and cannot be bypassed by retagging.

- [Architecture and mapping to the paper](docs/ARCHITECTURE.md) (Chinese)
- [Security boundaries and backup](docs/SECURITY.md) (Chinese)
- [Ontology and API conventions](docs/ONTOLOGY-API.md) (Chinese)
- [Self ontology and instrument catalog](docs/SELF-MODEL.md) (Chinese)
- [AI agent integration](docs/AGENT-INTEGRATION.en.md) ([OpenAPI](docs/agent-openapi.json))
- [Local models and routing](docs/LOCAL-AI.md) (Chinese)
- [Validation record](docs/VALIDATION.md) (Chinese)

## Development and verification

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;D:\prm\.tools\w64devkit\bin;$env:Path"
.\scripts\Start-Postgres.ps1 # or set PRM_DATABASE_URL beforehand
npm.cmd ci
npm.cmd run build
cargo build --locked
$env:PRM_DATABASE_SCHEMA = $null # tests derive their own schema per temp dir
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
npx.cmd playwright install chromium
npm.cmd run test:e2e
```

The browser tests start a dedicated server on port 4318, create a test account and synthetic data under `test-data/`, and write screenshots to `test-results/`. For frontend work, run `npm.cmd run dev` in a second terminal; Vite proxies `/api` to the Rust server on 4317.

## Data and scope

Business data and the encrypted mental snapshot live in PostgreSQL under one workspace schema and transaction. `data/` keeps the process lock and legacy raw SQLite files; old data is imported read-only into an empty PostgreSQL workspace on first launch, and the original files are no longer updated. Version 0.4 added the scene column to existing workspaces and synchronized ontology constraints while leaving records untouched (old self-assessments show as “legacy self-report”); 0.5 backfills entities and relations for existing agent connections (old connections keep working; reconnect in Digital space to get a relation). Full backups should use `pg_dump`; copying `data/` is not enough — see the security document.

A vault “read-only grant” binds the recipient's device public key to selected items, and only the owner can change it. The recipient opens `/shared` to obtain the public key and hand it to the owner, then reads with the grant id. Reading works only while the owner's vault is unlocked. Grants never include version history and cannot be elevated to write access.

Security uses Ed25519 request signing, content digests, timestamps and persisted replay protection. The browser holds a non-extractable private key through Web Crypto. External agents and the local model executor use separate, revocable tokens confined to fixed endpoints, and everything they produce awaits owner review.

The first release is single-machine and single-user: no multi-device sync, Windows Hello, messaging-network connectors, bank payments, full accounting or robot control. The PRP side is a foundation for resource and cash management and never adds stock value, cash flow and human worth together; the psychological catalog stores only the owner's scores, diagnoses nothing and never merges them into a single score. A real local model must be installed separately.

## License

MIT License — see [LICENSE](LICENSE).
