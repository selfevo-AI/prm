//! Connection registry for external AI agents: platform presets (Grok, Claude,
//! ChatGPT, Muse, Dify, …), the `ai_agent` entity properties, and the copy-ready
//! connection snippet shown after a token is issued. A connection opens a
//! relation between the self and the agent; the platform list is data, so new
//! agents only need an entry here plus a label in the UI.
use crate::{
    error::{require, Result},
    ontology::validate_optional,
};
use serde_json::{json, Value};

pub struct Platform {
    pub id: &'static str,
    pub label: &'static str,
    /// Suggested transport; the owner may still change it.
    pub protocol: &'static str,
    pub docs: &'static str,
    pub hint: &'static str,
}
pub const PLATFORMS: &[Platform] = &[
    Platform {
        id: "grok",
        label: "Grok（xAI）",
        protocol: "http",
        docs: "https://docs.x.ai",
        hint: "在 Grok 机器人、xAI 应用或自建工具里使用工具调用，把整理好的信息 POST 到 PRM 导入接口。",
    },
    Platform {
        id: "chatgpt",
        label: "ChatGPT（OpenAI）",
        protocol: "http",
        docs: "https://platform.openai.com/docs",
        hint: "用 GPTs Actions 或自建助手调用 PRM 的 OpenAPI 导入接口。",
    },
    Platform {
        id: "claude",
        label: "Claude（Anthropic）",
        protocol: "mcp",
        docs: "https://docs.anthropic.com",
        hint: "在 Claude Desktop 或 Claude Code 的 MCP 配置中挂载 prm-mcp.mjs，令牌填入 PRM_CONNECTION_TOKEN。",
    },
    Platform {
        id: "gemini",
        label: "Gemini（Google）",
        protocol: "http",
        docs: "https://ai.google.dev",
        hint: "在 Gemini 应用或 Google 的智能体框架里通过 HTTP 工具调用导入。",
    },
    Platform {
        id: "copilot",
        label: "Microsoft Copilot",
        protocol: "http",
        docs: "https://learn.microsoft.com/microsoft-copilot-studio",
        hint: "用 Copilot Studio 的 HTTP 操作调用 PRM 导入接口。",
    },
    Platform {
        id: "muse",
        label: "Muse（muse.ai）",
        protocol: "http",
        docs: "https://muse.ai/",
        hint: "把 muse.ai 会议助手整理出的纪要、任务与资源信息，通过 HTTP 导入接口 POST 到 PRM。",
    },
    Platform {
        id: "deepseek",
        label: "DeepSeek",
        protocol: "http",
        docs: "https://api-docs.deepseek.com",
        hint: "在基于 DeepSeek 的机器人里用函数调用（Function Calling）提交导入。",
    },
    Platform {
        id: "qwen",
        label: "通义千问（Qwen）",
        protocol: "http",
        docs: "https://help.aliyun.com/zh/model-studio",
        hint: "在通义千问或百炼应用中添加 HTTP 工具，调用 PRM 导入接口。",
    },
    Platform {
        id: "kimi",
        label: "Kimi（月之暗面）",
        protocol: "http",
        docs: "https://platform.moonshot.cn/docs",
        hint: "在 Kimi / Moonshot 开放平台的助手流程中调用 PRM 导入接口。",
    },
    Platform {
        id: "dify",
        label: "Dify",
        protocol: "http",
        docs: "https://docs.dify.ai",
        hint: "在 Dify 工作流中添加 HTTP 请求节点，指向 PRM 导入接口。",
    },
    Platform {
        id: "n8n",
        label: "n8n",
        protocol: "http",
        docs: "https://docs.n8n.io",
        hint: "在 n8n 中添加 HTTP Request 节点，指向 PRM 导入接口。",
    },
    Platform {
        id: "langchain",
        label: "LangChain / LangGraph",
        protocol: "http",
        docs: "https://python.langchain.com",
        hint: "用任意 HTTP 客户端或 LangChain 工具提交导入。",
    },
    Platform {
        id: "cursor",
        label: "Cursor",
        protocol: "mcp",
        docs: "https://docs.cursor.com",
        hint: "在 Cursor 的 MCP 设置中挂载 prm-mcp.mjs。",
    },
    Platform {
        id: "cline",
        label: "Cline",
        protocol: "mcp",
        docs: "https://docs.cline.bot",
        hint: "在 Cline 的 MCP 服务器配置中挂载 prm-mcp.mjs。",
    },
    Platform {
        id: "custom",
        label: "自定义 / 通用",
        protocol: "http",
        docs: "",
        hint: "任何能发本机 HTTP 请求或使用 MCP 的 Agent 都可以接入。",
    },
];
pub fn platform(id: &str) -> Option<&'static Platform> {
    PLATFORMS.iter().find(|p| p.id == id)
}
/// The platform presets as sent to the UI.
pub fn platforms_json() -> Vec<Value> {
    PLATFORMS
        .iter()
        .map(|p| {
            json!({"id": p.id, "label": p.label, "protocol": p.protocol, "docs": p.docs, "hint": p.hint})
        })
        .collect()
}
/// Validated `ai_agent` entity properties; empty platform means "custom".
pub fn agent_properties(platform_id: &str, protocol: &str, notes: &str) -> Result<Value> {
    let id = if platform_id.is_empty() {
        "custom"
    } else {
        platform_id
    };
    require(platform(id).is_some(), "接入平台不合法")?;
    validate_optional(notes, 2000, "备注过长")?;
    Ok(json!({"platform": id, "protocol": protocol, "notes": notes}))
}
/// A copy-ready connection snippet for an issued token.
pub fn snippet(protocol: &str, token: &str) -> String {
    if protocol == "mcp" {
        return serde_json::to_string_pretty(&json!({
            "mcpServers": {"prm": {
                "command": "node",
                "args": ["scripts/prm-mcp.mjs"],
                "env": {"PRM_CONNECTION_TOKEN": token, "PRM_URL": "http://127.0.0.1:4317"}
            }}
        }))
        .unwrap_or_default();
    }
    format!(
        "POST http://127.0.0.1:4317/api/agent/import\n\
         Authorization: Bearer {token}\n\
         X-PRM-Request: 1\n\
         Content-Type: application/json\n\n\
         {{\"items\": [{{\"kind\": \"task\", \"title\": \"…\"}}]}}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_ids_are_unique_and_custom_is_available() {
        for (i, p) in PLATFORMS.iter().enumerate() {
            assert!(!p.id.is_empty(), "{}", p.label);
            assert!(PLATFORMS[..i].iter().all(|o| o.id != p.id), "{}", p.id);
            assert!(["http", "mcp", "other"].contains(&p.protocol), "{}", p.id);
        }
        assert!(platform("custom").is_some());
        assert!(platform("grok").is_some());
        assert!(platform("").is_none());
        assert!(agent_properties("", "http", "").is_ok());
        assert!(agent_properties("muse", "http", "x").is_ok());
        assert!(agent_properties("nope", "http", "").is_err());
    }

    #[test]
    fn snippets_carry_the_token_and_protocol_shape() {
        let mcp = snippet("mcp", "prm_token");
        assert!(mcp.contains("\"PRM_CONNECTION_TOKEN\": \"prm_token\""));
        assert!(mcp.contains("prm-mcp.mjs"));
        let http = snippet("http", "prm_token");
        assert!(http.contains("Bearer prm_token"));
        assert!(http.contains("/api/agent/import"));
    }
}
