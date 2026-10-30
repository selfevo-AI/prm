import { locale, t } from "./i18n";
import { devicePublicKey, signatureHeaders } from "./security";
export interface Entity {
  id: string;
  kind: string;
  title: string;
  worlds: string[];
  values: string[];
  scenes: string[];
  properties: Record<string, unknown>;
  status: string;
  version: number;
  created_at: string;
  updated_at: string;
}
export interface Relation {
  id: string;
  subject: string;
  predicate: string;
  object: string;
}
export interface Audit {
  sequence: number;
  at: string;
  actor: string;
  action: string;
  target: string;
}
export interface CoreItem {
  id: string;
  kind: string;
  title: string;
  body: string;
  values: string[];
  version: number;
  updated_at: string;
  confirmed_at: string;
}
export interface Vault {
  version: number;
  items: CoreItem[];
  history: CoreItem[];
}
export interface Facet {
  id: string;
  label: string;
  description?: string;
}
export interface OntologyClass {
  id: string;
  label: string;
  category: string;
  world: string;
  worlds: string[];
  authority: string;
  storage: string;
  default_scenes: string[];
}
export interface Predicate {
  id: string;
  label: string;
  layer: string;
  owner_settable: boolean;
  agent_proposable: boolean;
  domain: string[];
  range: string[];
  signatures: { domain: string[]; range: string[] }[];
}
export interface Workspace {
  entities: Entity[];
  relations: Relation[];
  audit: Audit[];
  analysis_mode: string;
  ontology: {
    worlds: Facet[];
    value_axes: Facet[];
    scenes: Facet[];
    categories: Facet[];
    layers: Facet[];
    classes: OntologyClass[];
    predicates: Predicate[];
  };
}
export interface Metric {
  id: string;
  label: string;
  unit: string;
  min: number;
  max: number;
  higher: "better" | "worse" | "neutral";
}
export interface Instrument {
  id: string;
  abbr: string;
  name: string;
  domain: "body" | "psychology" | "relationship";
  family: string;
  construct: string;
  items: number;
  response: string;
  citation: string;
  scenes: string[];
  metrics: Metric[];
}
export interface SelfCatalog {
  instruments: Instrument[];
  stances: Facet[];
  identity_categories: Facet[];
  relationship_types: { id: string; label: string; scene: string }[];
  interaction_channels: Facet[];
  notice: string;
}
export type Dialog = {
  type: string;
  entity?: Entity;
  core?: CoreItem;
  delta?: number;
} | null;
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}
export async function api<T>(path: string, body?: unknown): Promise<T> {
  if (path === "/auth/login" || path === "/auth/setup") {
    body = { ...(body as object), public_key: await devicePublicKey() };
  }
  const encoded = body === undefined ? "" : JSON.stringify(body);
  const method = body === undefined ? "GET" : "POST";
  const proof = path === "/auth/status" ? {} : await signatureHeaders(`/api${path}`, method, encoded);
  const response = await fetch(`/api${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json", "X-PRM-Request": "1", ...proof },
    credentials: "same-origin",
    cache: "no-store",
    ...(body === undefined ? {} : { body: encoded }),
  });
  const result = await response
    .json()
    .catch(() => ({ error: t("服务返回了无效响应") }));
  if (!response.ok)
    throw new ApiError(result.error || t("请求失败，请稍后重试"), response.status);
  return result as T;
}
export const SCENES = ["self", "emotion", "life", "work", "health", "learning", "finance", "social"];
export const labels: Record<string, string> = {
  principle: "原则",
  goal: "目标",
  reflection: "反思",
  directive: "执行方向",
  channel: "会话",
  message: "消息",
  note: "知识条目",
  proposal: "建议",
  task: "任务",
  resource: "资源",
  location: "库位",
  person: "人物",
  identity: "自我认同",
  relationship: "人际关系",
  interaction: "互动记录",
  self_profile: "自我本体",
  self_relation: "自我关系",
  body_metric: "生理自我指标",
  psychological_metric: "心理自我认定",
  supplier: "供应商",
  purchase_order: "采购订单",
  purchase_order_line: "采购订单行",
  agent_run: "路由运行",
  agent_import: "Agent 导入",
  ai_agent: "AI Agent",
  model_endpoint: "本地模型",
  route_rule: "路由规则",
  cash_entry: "现金流水",
  stock_movement: "库存流水",
  need: "需要",
  desire: "欲望",
  existence: "存在",
  mental: "思维世界",
  digital: "数字世界",
  physical: "物理世界",
  self: "自我本体",
  "scene:self": "自我",
  "scene:emotion": "情感",
  "scene:life": "生活",
  "scene:work": "工作",
  "scene:health": "健康",
  "scene:learning": "学习",
  "scene:finance": "财务",
  "scene:social": "社交",
  sleep_duration: "睡眠时长",
  sleep_quality: "睡眠质量",
  energy: "精力",
  resting_heart_rate: "静息心率",
  activity: "活动量",
  pain: "疼痛感",
  self_esteem: "自尊",
  autonomy: "自主感",
  competence: "胜任感",
  relatedness: "联结感",
  wellbeing: "主观幸福感",
  stress: "压力感",
  resilience: "心理韧性",
  emotional_awareness: "情绪觉察",
  todo: "待开始",
  doing: "进行中",
  done: "已完成",
  cancelled: "已取消",
  active: "有效",
  pending: "待确认",
  approved: "已采纳",
  rejected: "已拒绝",
  revoked: "已撤回",
  observed: "已记录",
  posted: "已过账",
  paused: "暂停",
  ended: "已结束",
  disabled: "已停用",
  draft: "草稿",
  ordered: "已下单",
  partially_received: "部分收货",
  received: "已收货",
  closed: "已完结",
  open: "未收齐",
  queued: "排队中",
  processing: "处理中",
  processed: "已生成候选",
  running: "执行中",
  completed: "已完成",
  failed: "失败",
  pending_review: "待审（旧版）",
  implements: "落实",
  uses: "使用",
  references: "关联",
  derived_from: "源于",
  belongs_to: "属于",
  records: "记录",
  describes_self: "描述自我",
  relates_to_self: "与自我关系",
  measures: "测量",
  supports: "支持",
  influences: "影响",
  supplies: "供应",
  contains: "包含",
  orders: "订购",
  receives: "收货",
  pays: "付款",
  stored_at: "存放于",
  located_in: "位于",
  involves: "涉及",
  occurs_in: "发生于",
  processes: "处理",
  from_agent: "来自",
  ai_assistant: "AI 助理",
  ai_companion: "AI 伙伴",
  ai_mentor: "AI 导师",
  grok: "Grok（xAI）",
  chatgpt: "ChatGPT（OpenAI）",
  claude: "Claude（Anthropic）",
  gemini: "Gemini（Google）",
  copilot: "Microsoft Copilot",
  muse: "Muse（muse.ai）",
  deepseek: "DeepSeek",
  qwen: "通义千问（Qwen）",
  kimi: "Kimi（月之暗面）",
  dify: "Dify",
  n8n: "n8n",
  langchain: "LangChain / LangGraph",
  cursor: "Cursor",
  cline: "Cline",
  custom: "自定义 / 通用",
  http: "HTTP / OpenAPI",
  mcp: "MCP 工具桥",
  other: "其他",
  consumable: "消耗品",
  asset: "资产",
  receipt: "入库",
  issue: "出库",
  adjustment: "盘点调整",
  opening: "期初",
  create_task: "建议任务",
  create_entity: "纳入本体",
  create_relation: "建立关系",
  ollama: "Ollama",
  openai_compatible: "OpenAI 兼容",
  "offline-rules": "离线规则",
};
export const sceneLabel = (scene: string) => t(labels[`scene:${scene}`] || scene);
/** A taxonomy key translated through the shared label table. */
export const label = (key: string) => t(labels[key] ?? key);
export const time = (value: string) =>
  new Date(value).toLocaleString(locale() === "zh" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
export const today = () => new Date().toLocaleDateString("en-CA");
export const money = (cents: number) =>
  new Intl.NumberFormat(locale() === "zh" ? "zh-CN" : "en-US", {
    style: "currency",
    currency: "CNY",
  }).format(cents / 100);
export const prop = (entity: Entity, key: string) =>
  String(entity.properties[key] ?? "");
export const num = (entity: Entity, key: string) =>
  Number(entity.properties[key] ?? 0);
export const visible = (entity: Entity) => entity.properties.agent_visible === true;
/** Yuan text (at most two decimals) to integer cents. */
export function cents(amount: string): number {
  if (!/^\d+(\.\d{1,2})?$/.test(amount.trim()))
    throw new Error(t("金额最多保留两位小数"));
  const [whole, fraction = ""] = amount.trim().split(".");
  const value = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  if (!Number.isSafeInteger(value)) throw new Error(t("金额超出范围"));
  return value;
}
/** Days since a YYYY-MM-DD date (or an ISO timestamp). */
export function daysSince(value: string): number {
  const date = new Date(value.length === 10 ? `${value}T00:00:00` : value);
  return Math.floor((Date.now() - date.getTime()) / 86_400_000);
}
