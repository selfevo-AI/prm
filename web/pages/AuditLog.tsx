import type { Workspace } from "../api";
import { locale, t } from "../i18n";

const descriptions: Record<string, string> = {
  "account.created": "建立账户",
  "auth.login": "账户登录",
  "auth.failed": "口令验证失败",
  "vault.unlocked": "解锁思维库",
  "vault.confirmed": "确认思维条目",
  "vault.reviewed": "复核思维条目",
  "vault.read_granted": "建立只读授权",
  "vault.read_revoked": "撤销只读授权",
  "vault.shared_read": "接收者读取授权内容",
  "directive.published": "发布执行方向",
  "directive.revoked": "撤回执行方向授权",
  "entity.created": "建立业务实体",
  "task.status_changed": "更新任务状态",
  "stock.posted": "登记库存变动",
  "cash.posted": "登记现金流水",
  "purchase.status_changed": "更新采购订单状态",
  "purchase.received": "采购收货",
  "purchase.paid": "采购付款",
  "message.created": "发送消息",
  "proposal.created": "生成建议",
  "proposal.approved": "采纳建议",
  "proposal.rejected": "拒绝建议",
  "relation.created": "建立本体关系",
  "demo.created": "添加演示数据",
  "self.assessment_recorded": "记录自我测评",
  "self.identity_recorded": "记录自我认同",
  "self.relation_recorded": "记录自我关系",
  "self.visibility_changed": "调整 Agent 可见性",
  "relationship.created": "建立人际关系",
  "relationship.updated": "调整人际关系",
  "relationship.interaction_recorded": "记录互动",
  "agent.connection_created": "签发外部 Agent 令牌",
  "agent.connection_revoked": "撤销外部 Agent 令牌",
  "agent.model_registered": "登记本地模型",
  "agent.route_created": "添加路由规则",
  "agent.config_changed": "调整路由配置",
  "agent.import_received": "接收 Agent 导入",
  "agent.run_queued": "导入进入本地模型队列",
  "agent.run_claimed": "执行器领取任务",
  "agent.run_completed": "路由处理完成",
  "agent.run_failed": "路由处理失败",
  "database.migrated": "迁移到 PostgreSQL",
};
const actors: Record<string, string> = {
  owner: "本人",
  agent: "离线助理",
  executor: "本地执行器",
  external: "外部 Agent",
  reader: "只读接收者",
};

export function AuditLog({ workspace }: { workspace: Workspace }) {
  return (
    <>
      <div className="section-heading">
        <h2>
          {t("最近活动")}
          <span className="count">{workspace.audit.length}</span>
        </h2>
        <span className="section-meta">{t("最近 200 条")}</span>
      </div>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t("时间")}</th>
              <th>{t("执行者")}</th>
              <th>{t("操作")}</th>
              <th>{t("对象")}</th>
            </tr>
          </thead>
          <tbody>
            {workspace.audit.map((a) => (
              <tr key={a.sequence}>
                <td className="muted nowrap">
                  {new Date(a.at).toLocaleString(locale() === "zh" ? "zh-CN" : "en-US", {
                    month: "2-digit",
                    day: "2-digit",
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </td>
                <td>
                  <span className={`actor ${a.actor}`}>
                    <span className="mini-dot" />
                    {t(actors[a.actor] || "未认证")}
                  </span>
                </td>
                <td>{t(descriptions[a.action] || a.action)}</td>
                <td>
                  {workspace.entities.find((e) => e.id === a.target)?.title ||
                    (a.target === "vault"
                      ? t("私密思维库")
                      : a.target === "account"
                        ? t("本地账户")
                        : a.target === "workspace"
                          ? t("工作区")
                          : a.target.slice(0, 8))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
