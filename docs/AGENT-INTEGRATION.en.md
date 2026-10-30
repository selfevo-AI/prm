# External AI agent integration

External agents can hand organized information to PRM: resources (items, devices, consumables), relations (who supplies what, what relates to what), messages, knowledge, people and tasks. PRM stores each submission as an immutable “import envelope”, routes it to the offline rules or a local model, and turns the outcome into proposals that only the owner can accept. A connection token **can only import** — it never reads anything from PRM.

Mainstream agents all fit: the platform presets cover Grok (xAI), Claude (Anthropic), ChatGPT (OpenAI), Gemini (Google), Microsoft Copilot, Muse (muse.ai), DeepSeek, Qwen (Alibaba), Kimi (Moonshot), Dify, n8n, LangChain / LangGraph, Cursor, Cline, plus “Custom / Generic”. Any agent that can make local HTTP requests or speak MCP works the same way.

## 1. Connecting is establishing a relation

In Digital space · Agent connections · External agent connections, pick a platform, fill in the connection name, transport, relation type (AI assistant / AI companion / AI mentor) and relation scenes (life, work, learning, …; none picked means the relation type's default). “Issue token” does three things at once:

1. The agent is registered as a digital-world entity (platform, transport and notes are recorded).
2. **A relation between you and this AI agent is created** — relations with AI agents are part of the relations network: review and adjust them there, rate closeness and satisfaction, log interactions, and decide record by record whether the local model may read them.
3. A one-time `prm_…` token is generated (shown once; PRM stores only its hash) together with a copy-ready connection snippet.

Revoke the token when you no longer need it — it stops working immediately, and the relation stays in the relations network for you to keep. Connections created by earlier versions have no entity or relation attached; reconnect once to get the full relation record.

## 2. HTTP interface (Grok bots, Dify, n8n, LangChain, custom agents)

The interface is described in [agent-openapi.json](agent-openapi.json) (OpenAPI 3.1) and can be imported into platforms that consume OpenAPI. The server only listens on the local loopback address, so callers must run on the same machine.

```powershell
$token = "prm_..."
$body = @'
{
  "scenes": ["work"],
  "items": [
    {"ref": "r1", "kind": "resource", "title": "Meeting room projector", "attributes": {"unit": "unit", "location": "Meeting room"}},
    {"ref": "p1", "kind": "person", "title": "Manager Wang", "body": "Project contact"},
    {"kind": "relation", "subject": "p1", "predicate": "references", "object": "r1"},
    {"kind": "task", "title": "Send the weekly report by Friday"}
  ]
}
'@
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:4317/api/agent/import `
  -Headers @{ Authorization = "Bearer $token"; "X-PRM-Request" = "1" } `
  -ContentType "application/json; charset=utf-8" -Body ([Text.Encoding]::UTF8.GetBytes($body))
```

OpenAI-style conversations are accepted directly:

```json
{"messages": [{"role": "user", "content": "Remind me to bring the health report on Wednesday"}, {"role": "assistant", "content": "Okay"}]}
```

Only `user` and `assistant` messages are imported, and they become to-do candidates. Provide either `items` or `messages` — up to 100 entries, request body at most 128 KiB.

Query status with `GET /api/agent/imports` (the latest 50) or `GET /api/agent/imports/{id}`: processing status and pending/accepted/rejected proposal counts, never any content. Every import is traceable back to its agent entity through the `from_agent` relation.

## 3. MCP tool bridge (Claude Desktop, Cursor, Cline, …)

`scripts/prm-mcp.mjs` is a dependency-free MCP stdio server exposing two tools:

- `prm_import`: the same parameters as HTTP import (`items` or `messages`, `scenes`, `conversation`).
- `prm_import_status`: query an import's status.

Add it to the client's MCP configuration (adjust the path):

```json
{
  "mcpServers": {
    "prm": {
      "command": "node",
      "args": ["D:\\prm\\scripts\\prm-mcp.mjs"],
      "env": {"PRM_CONNECTION_TOKEN": "prm_...", "PRM_URL": "http://127.0.0.1:4317"}
    }
  }
}
```

Pick “MCP tool bridge” as the transport when issuing the token so the connection list stays readable; after issuing, the UI shows the full MCP config with the token filled in, ready to copy.

## 4. Processing and review

After an import:

- Without a matching routing rule, the offline rules produce proposals immediately (status “Proposals created”).
- With a local-model rule, the import is queued and processed by the local model executor under the “self ontology core principles” (see [LOCAL-AI.md](LOCAL-AI.md), in Chinese).

Proposals appear in the Decisions page; filter the source by “External agent import”. Each shows its action (suggest task / add entity / create relation), attributes, rationale, referenced core principles and the source import. A relation that references another proposal from the same import requires that proposal to be accepted first. Rejecting never deletes the import — import records always remain under Agent connections · Imports.
