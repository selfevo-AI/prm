import { useState } from "react";
import {
  ArrowDownLeft,
  ArrowUpRight,
  Boxes,
  ClipboardList,
  PackageCheck,
  Plus,
  Scale,
  Wallet,
} from "lucide-react";
import { api, label, money, num, prop, type Entity } from "../api";
import { Empty, Scenes, StatusBadge, Values } from "../components";
import { find, type PageContext } from "../context";
import { locale, t } from "../i18n";

const tabs = [
  ["resource", "资源台账"],
  ["location", "库位"],
  ["supplier", "供应商"],
  ["purchase_order", "采购订单"],
  ["stock_movement", "库存流水"],
  ["cash_entry", "现金流水"],
] as const;
type Tab = (typeof tabs)[number][0];
const createLabel: Partial<Record<Tab, string>> = {
  resource: "登记资源",
  location: "新建库位",
  supplier: "新建供应商",
  purchase_order: "新建采购订单",
  cash_entry: "记一笔",
};
const dialogType: Partial<Record<Tab, string>> = {
  resource: "resource",
  location: "location",
  supplier: "supplier",
  purchase_order: "purchase_order",
  cash_entry: "cash",
};

export function Physical({ ctx }: { ctx: PageContext }) {
  const [tab, setTab] = useState<Tab>("resource");
  const { byKind, filter, onDialog, entities } = ctx;
  const list = filter(byKind(tab));
  return (
    <>
      <div className="tabs">
        {tabs.map(([value, title]) => (
          <button
            key={value}
            className={tab === value ? "active" : ""}
            onClick={() => setTab(value)}
          >
            {t(title)}
          </button>
        ))}
      </div>
      <div className="section-heading">
        <h2>
          {t(tabs.find(([value]) => value === tab)![1])}
          <span className="count">{list.length}</span>
        </h2>
        {createLabel[tab] && (
          <button
            className="button primary"
            onClick={() => onDialog({ type: dialogType[tab]! })}
          >
            <Plus size={16} />
            {t(createLabel[tab])}
          </button>
        )}
      </div>
      {tab === "cash_entry" && (
        <div className="cash-summary">
          <span>
            {t("已记录收入")}{" "}
            <strong>
              {money(
                list
                  .filter((e) => prop(e, "direction") === "income")
                  .reduce((s, e) => s + num(e, "amount_cents"), 0),
              )}
            </strong>
          </span>
          <span>
            {t("已记录支出")}{" "}
            <strong>
              {money(
                list
                  .filter((e) => prop(e, "direction") === "expense")
                  .reduce((s, e) => s + num(e, "amount_cents"), 0),
              )}
            </strong>
          </span>
          <span className="muted">{t("人民币 CNY")}</span>
        </div>
      )}
      {!list.length ? (
        <Empty title={t("还没有相关记录")} />
      ) : tab === "purchase_order" ? (
        <Orders ctx={ctx} orders={list} />
      ) : (
        <div className="table-wrap">
          <table>
            {tab === "resource" && <ResourceRows list={list} ctx={ctx} />}
            {tab === "location" && (
              <>
                <thead>
                  <tr>
                    <th>{t("库位")}</th>
                    <th>{t("编码")}</th>
                    <th>{t("上级库位")}</th>
                    <th>{t("存放资源")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map((e) => (
                    <tr key={e.id}>
                      <td>{e.title}</td>
                      <td className="mono">{prop(e, "code")}</td>
                      <td>{find(entities, prop(e, "parent_id"))?.title || "—"}</td>
                      <td>
                        {byKind("resource").filter((r) => prop(r, "location_id") === e.id).length}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </>
            )}
            {tab === "supplier" && (
              <>
                <thead>
                  <tr>
                    <th>{t("供应商")}</th>
                    <th>{t("联系方式")}</th>
                    <th>{t("税号")}</th>
                    <th>{t("供应物料")}</th>
                    <th>{t("采购订单")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map((e) => (
                    <tr key={e.id}>
                      <td>{e.title}</td>
                      <td>{prop(e, "contact") || "—"}</td>
                      <td className="mono">{prop(e, "tax_id") || "—"}</td>
                      <td>
                        {byKind("resource").filter((r) => prop(r, "supplier_id") === e.id).length}
                      </td>
                      <td>
                        {byKind("purchase_order").filter((o) => prop(o, "supplier_id") === e.id).length}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </>
            )}
            {tab === "stock_movement" && (
              <>
                <thead>
                  <tr>
                    <th>{t("时间")}</th>
                    <th>{t("资源")}</th>
                    <th>{t("类型")}</th>
                    <th>{t("摘要")}</th>
                    <th>{t("变动数量")}</th>
                    <th>{t("结存数量")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map((e) => (
                    <tr key={e.id}>
                      <td className="muted nowrap">
                        {new Date(e.created_at).toLocaleString(
                          locale() === "zh" ? "zh-CN" : "en-US",
                          {
                            month: "2-digit",
                            day: "2-digit",
                            hour: "2-digit",
                            minute: "2-digit",
                          },
                        )}
                      </td>
                      <td>{find(entities, prop(e, "resource_id"))?.title || t("未知资源")}</td>
                      <td>
                        {movementLabel(e)}
                      </td>
                      <td>{e.title}</td>
                      <td className={num(e, "delta") > 0 ? "green" : "coral"}>
                        {num(e, "delta") > 0 ? "+" : ""}
                        {num(e, "delta")} {t(prop(e, "unit"))}
                      </td>
                      <td>
                        {num(e, "balance")} {t(prop(e, "unit"))}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </>
            )}
            {tab === "cash_entry" && (
              <>
                <thead>
                  <tr>
                    <th>{t("日期")}</th>
                    <th>{t("摘要")}</th>
                    <th>{t("分类")}</th>
                    <th>{t("评价维度")}</th>
                    <th>{t("金额")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.map((e) => (
                    <tr key={e.id}>
                      <td className="muted nowrap">{prop(e, "date")}</td>
                      <td>{e.title}</td>
                      <td>{prop(e, "category")}</td>
                      <td>
                        <Values values={e.values} />
                      </td>
                      <td
                        className={
                          prop(e, "direction") === "income"
                            ? "green nowrap"
                            : "nowrap"
                        }
                      >
                        {prop(e, "direction") === "income" ? "+" : "-"}
                        {money(num(e, "amount_cents"))}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </>
            )}
          </table>
        </div>
      )}
    </>
  );
}

function movementLabel(e: Entity) {
  const type = prop(e, "movement_type");
  return type
    ? label(type)
    : num(e, "delta") > 0
      ? t("入库")
      : t("出库");
}

function ResourceRows({ list, ctx }: { list: Entity[]; ctx: PageContext }) {
  const { onDialog } = ctx;
  return (
    <>
      <thead>
        <tr>
          <th>{t("资源名称")}</th>
          <th>{t("位置")}</th>
          <th>{t("可用库存")}</th>
          <th>{t("最低库存")}</th>
          <th>{t("评价维度")}</th>
          <th>{t("出入库")}</th>
        </tr>
      </thead>
      <tbody>
        {list.map((e) => (
          <tr key={e.id}>
            <td>
              <div className="resource-name">
                <span className="resource-icon">
                  <Boxes size={18} />
                </span>
                <div>
                  <button
                    className="title-button"
                    onClick={() => onDialog({ type: "detail", entity: e })}
                  >
                    {e.title}
                  </button>
                  <div className="resource-meta">
                    {prop(e, "item_type") === "asset" && (
                      <span className="tag">{label(prop(e, "item_type"))}</span>
                    )}
                    {prop(e, "sku") && <span className="mono muted">{prop(e, "sku")}</span>}
                    {prop(e, "supplier") && <span className="muted">{prop(e, "supplier")}</span>}
                  </div>
                </div>
              </div>
            </td>
            <td>{prop(e, "location")}</td>
            <td>
              <strong className={num(e, "quantity") < num(e, "minimum") ? "coral" : ""}>
                {num(e, "quantity")}
              </strong>{" "}
              <span className="muted">{t(prop(e, "unit"))}</span>
            </td>
            <td>{num(e, "minimum")}</td>
            <td>
              <Values values={e.values} />
            </td>
            <td>
              <div className="row-actions">
                <button
                  className="icon-button outlined"
                  title={t("入库")}
                  aria-label={t("{title}入库", { title: e.title })}
                  onClick={() => onDialog({ type: "movement", entity: e, delta: 1 })}
                >
                  <ArrowDownLeft size={17} />
                </button>
                <button
                  className="icon-button outlined"
                  title={t("出库")}
                  aria-label={t("{title}出库", { title: e.title })}
                  onClick={() => onDialog({ type: "movement", entity: e, delta: -1 })}
                >
                  <ArrowUpRight size={17} />
                </button>
                <button
                  className="icon-button outlined"
                  title={t("盘点")}
                  aria-label={t("{title}盘点", { title: e.title })}
                  onClick={() => onDialog({ type: "adjust", entity: e })}
                >
                  <Scale size={16} />
                </button>
              </div>
            </td>
          </tr>
        ))}
      </tbody>
    </>
  );
}

function Orders({ ctx, orders }: { ctx: PageContext; orders: Entity[] }) {
  const { byKind, onDialog, run, busy, entities } = ctx;
  const setStatus = (order: Entity, status: string, message: string) =>
    void run(
      () =>
        api(`/purchase-orders/${order.id}/status`, {
          status,
          expected_version: order.version,
        }),
      message,
    );
  return (
    <div className="order-list">
      {orders.map((order) => {
        const lines = byKind("purchase_order_line").filter(
          (l) => prop(l, "purchase_order_id") === order.id,
        );
        const status = order.status === "active" ? "draft" : order.status;
        const total = num(order, "total_cents");
        const paid = num(order, "paid_cents");
        const received = lines.every((l) => num(l, "received_quantity") === 0);
        return (
          <article className="order-card" key={order.id}>
            <header>
              <span className="world-icon physical">
                <ClipboardList size={20} />
              </span>
              <div>
                <h2>{order.title}</h2>
                <span className="muted small-text">
                  {prop(order, "supplier") ||
                    find(entities, prop(order, "supplier_id"))?.title ||
                    t("未指定供应商")}
                  {prop(order, "expected_date") &&
                    t(" · 预计 {date}", { date: prop(order, "expected_date") })}
                </span>
              </div>
              <StatusBadge status={status} />
            </header>
            {lines.length ? (
              <div className="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th>{t("物料")}</th>
                      <th>{t("订购", undefined, "Ordered")}</th>
                      <th>{t("已收")}</th>
                      <th>{t("单价")}</th>
                      <th>{t("小计")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {lines.map((line) => (
                      <tr key={line.id}>
                        <td>{prop(line, "resource_title") || line.title}</td>
                        <td>
                          {num(line, "quantity")} {t(prop(line, "unit"))}
                        </td>
                        <td className={num(line, "received_quantity") >= num(line, "quantity") ? "green" : ""}>
                          {num(line, "received_quantity")}
                        </td>
                        <td>{money(num(line, "unit_cost_cents"))}</td>
                        <td>{money(num(line, "line_total_cents"))}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="muted small-text">{t("尚未添加订单行")}</p>
            )}
            <div className="order-totals">
              <span>
                {t("订单金额")} <strong>{money(total)}</strong>
              </span>
              <span>
                {t("已付款")} <strong>{money(paid)}</strong>
              </span>
              <Scenes scenes={order.scenes} />
            </div>
            <footer>
              {status === "draft" && (
                <>
                  <button className="button secondary" disabled={busy} onClick={() => onDialog({ type: "order_line", entity: order })}>
                    <Plus size={14} />
                    {t("添加订单行")}
                  </button>
                  <button className="button secondary" disabled={busy} onClick={() => setStatus(order, "cancelled", t("采购订单已取消"))}>
                    {t("取消订单")}
                  </button>
                  <button
                    className="button primary"
                    disabled={busy || !lines.length}
                    onClick={() => setStatus(order, "ordered", t("已确认下单"))}
                  >
                    {t("确认下单")}
                  </button>
                </>
              )}
              {["ordered", "partially_received"].includes(status) && (
                <button className="button primary" disabled={busy} onClick={() => onDialog({ type: "receive", entity: order })}>
                  <PackageCheck size={14} />
                  {t("收货", undefined, "Receive")}
                </button>
              )}
              {status === "ordered" && received && paid === 0 && (
                <button className="button secondary" disabled={busy} onClick={() => setStatus(order, "cancelled", t("采购订单已取消"))}>
                  {t("取消订单")}
                </button>
              )}
              {["ordered", "partially_received", "received", "closed"].includes(status) && paid < total && (
                <button className="button secondary" disabled={busy} onClick={() => onDialog({ type: "pay", entity: order })}>
                  <Wallet size={14} />
                  {t("付款", undefined, "Pay")}
                </button>
              )}
              {["ordered", "partially_received", "received"].includes(status) && (
                <button className="button secondary" disabled={busy} onClick={() => setStatus(order, "closed", t("采购订单已完结"))}>
                  {t("完结")}
                </button>
              )}
            </footer>
          </article>
        );
      })}
    </div>
  );
}
