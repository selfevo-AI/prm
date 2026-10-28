import { useState, type FormEvent } from "react";
import { api, cents, money, num, prop, today, type Dialog, type Workspace } from "../api";
import { Modal, SceneInput, SelectField, TextField } from "../components";

const types = ["location", "supplier", "purchase_order", "order_line", "receive", "pay", "adjust"];
export const isPrpDialog = (type: string) => types.includes(type);
const titles: Record<string, string> = {
  location: "新建库位",
  supplier: "新建供应商",
  purchase_order: "新建采购订单",
  order_line: "添加订单行",
  receive: "采购收货",
  pay: "采购付款",
  adjust: "库存盘点",
};

export function PrpDialog({
  dialog,
  workspace,
  onClose,
  onSaved,
}: {
  dialog: NonNullable<Dialog>;
  workspace: Workspace;
  onClose: () => void;
  onSaved: (message: string) => Promise<void>;
}) {
  const { type, entity } = dialog;
  const of = (kind: string) => workspace.entities.filter((e) => e.kind === kind);
  const lines = entity
    ? of("purchase_order_line").filter((l) => prop(l, "purchase_order_id") === entity.id)
    : [];
  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [parent, setParent] = useState("");
  const [notes, setNotes] = useState("");
  const [contact, setContact] = useState("");
  const [taxId, setTaxId] = useState("");
  const [supplier, setSupplier] = useState("");
  const [expected, setExpected] = useState("");
  const [scenes, setScenes] = useState<string[]>([]);
  const [resource, setResource] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [unitCost, setUnitCost] = useState("");
  const [received, setReceived] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      lines.map((l) => [l.id, String(Math.max(0, num(l, "quantity") - num(l, "received_quantity")))]),
    ),
  );
  const outstanding = entity ? num(entity, "total_cents") - num(entity, "paid_cents") : 0;
  const [amount, setAmount] = useState(outstanding > 0 ? (outstanding / 100).toFixed(2) : "");
  const [date, setDate] = useState(today());
  const [counted, setCounted] = useState(entity ? String(num(entity, "quantity")) : "0");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      let message = "已保存";
      if (type === "location")
        await api("/entities", {
          kind: "location",
          title: name,
          scenes,
          properties: { code, parent_id: parent, notes },
        });
      else if (type === "supplier")
        await api("/entities", {
          kind: "supplier",
          title: name,
          scenes,
          properties: { contact, tax_id: taxId, notes },
        });
      else if (type === "purchase_order")
        await api("/entities", {
          kind: "purchase_order",
          title: name,
          scenes,
          properties: { supplier_id: supplier, expected_date: expected, notes },
        });
      else if (type === "order_line") {
        const item = of("resource").find((r) => r.id === resource);
        if (!item) throw new Error("请选择物料");
        await api("/entities", {
          kind: "purchase_order_line",
          title: `${item.title} × ${quantity}`,
          scenes: entity!.scenes,
          properties: {
            purchase_order_id: entity!.id,
            resource_id: resource,
            quantity: Number(quantity),
            unit_cost_cents: unitCost.trim() ? cents(unitCost) : 0,
          },
        });
        message = "订单行已添加";
      } else if (type === "receive") {
        const chosen = lines
          .map((l) => ({ line_id: l.id, quantity: Number(received[l.id] || 0) }))
          .filter((l) => l.quantity > 0);
        if (!chosen.length) throw new Error("请填写本次收货数量");
        await api(`/purchase-orders/${entity!.id}/receive`, {
          lines: chosen,
          note: notes,
          expected_version: entity!.version,
        });
        message = "收货已入库";
      } else if (type === "pay") {
        await api(`/purchase-orders/${entity!.id}/pay`, {
          amount_cents: cents(amount),
          date,
          title: name,
          expected_version: entity!.version,
        });
        message = "付款已记入现金流水";
      } else if (type === "adjust") {
        const delta = Number(counted) - num(entity!, "quantity");
        if (!Number.isInteger(delta) || delta === 0) throw new Error("盘点数量没有变化");
        await api(`/resources/${entity!.id}/movement`, {
          delta,
          reason: notes || "库存盘点",
          movement_type: "adjustment",
          expected_version: entity!.version,
        });
        message = "盘点差异已过账";
      }
      await onSaved(message);
    } catch (reason) {
      setError((reason as Error).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal title={titles[type]} onClose={onClose} onSubmit={submit} busy={busy} error={error}>
      {type === "location" && (
        <>
          <TextField label="库位名称" value={name} onChange={setName} placeholder="例如：书房 · 二层柜" />
          <div className="form-grid">
            <TextField label="编码" value={code} onChange={setCode} maxLength={40} required={false} />
            <SelectField
              label="上级库位"
              value={parent}
              onChange={setParent}
              placeholder="无"
              options={of("location").map((l) => ({ value: l.id, label: l.title }))}
            />
          </div>
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={1000} required={false} />
        </>
      )}
      {type === "supplier" && (
        <>
          <TextField label="供应商名称" value={name} onChange={setName} />
          <div className="form-grid">
            <TextField label="联系方式" value={contact} onChange={setContact} maxLength={200} required={false} />
            <TextField label="税号" value={taxId} onChange={setTaxId} maxLength={80} required={false} />
          </div>
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={1000} required={false} />
        </>
      )}
      {type === "purchase_order" && (
        <>
          <TextField label="订单名称" value={name} onChange={setName} placeholder="例如：十一月家居补货" />
          <div className="form-grid">
            <SelectField
              label="供应商"
              value={supplier}
              onChange={setSupplier}
              placeholder="暂不指定"
              options={of("supplier").map((s) => ({ value: s.id, label: s.title }))}
            />
            <TextField label="预计到货" type="date" value={expected} onChange={setExpected} required={false} />
          </div>
          <TextField label="备注" value={notes} onChange={setNotes} maxLength={4000} required={false} />
          <p className="muted small-text">订单建立后为草稿，添加订单行后再确认下单。</p>
        </>
      )}
      {["location", "supplier", "purchase_order"].includes(type) && (
        <SceneInput scenes={scenes} onChange={setScenes} legend="人生场景（不选则按类型默认）" />
      )}
      {type === "order_line" && (
        <>
          <SelectField
            label="物料"
            value={resource}
            required
            onChange={(value) => {
              setResource(value);
              const item = of("resource").find((r) => r.id === value);
              if (item && num(item, "unit_cost_cents") > 0)
                setUnitCost((num(item, "unit_cost_cents") / 100).toFixed(2));
              if (item && num(item, "reorder_qty") > 0) setQuantity(String(num(item, "reorder_qty")));
            }}
            placeholder="选择物料"
            options={of("resource").map((r) => ({ value: r.id, label: `${r.title}（${prop(r, "unit")}）` }))}
          />
          <div className="form-grid">
            <TextField label="数量" type="number" min={1} step={1} value={quantity} onChange={setQuantity} />
            <TextField label="单价（元）" value={unitCost} onChange={setUnitCost} maxLength={20} required={false} />
          </div>
        </>
      )}
      {type === "receive" && (
        <>
          {lines
            .filter((l) => num(l, "received_quantity") < num(l, "quantity"))
            .map((l) => (
              <TextField
                key={l.id}
                label={`${prop(l, "resource_title") || l.title} 本次收货（未收 ${num(l, "quantity") - num(l, "received_quantity")}）`}
                type="number"
                min={0}
                max={num(l, "quantity") - num(l, "received_quantity")}
                step={1}
                required={false}
                value={received[l.id] || ""}
                onChange={(value) => setReceived({ ...received, [l.id]: value })}
              />
            ))}
          <TextField label="收货备注" value={notes} onChange={setNotes} maxLength={300} required={false} />
          <p className="muted small-text">每一行生成一条“入库”流水，并按订单单价更新物料成本。</p>
        </>
      )}
      {type === "pay" && (
        <>
          <p className="muted small-text">
            订单金额 {money(num(entity!, "total_cents"))}，已付 {money(num(entity!, "paid_cents"))}，未付 {money(outstanding)}。
          </p>
          <div className="form-grid">
            <TextField label="付款金额（元）" value={amount} onChange={setAmount} maxLength={20} />
            <TextField label="付款日期" type="date" value={date} onChange={setDate} />
          </div>
          <TextField label="摘要" value={name} onChange={setName} required={false} placeholder={`采购付款：${entity?.title}`} />
        </>
      )}
      {type === "adjust" && (
        <>
          <p className="muted small-text">
            账面库存 {num(entity!, "quantity")} {prop(entity!, "unit")}。填写实际清点数量，差异记为“盘点调整”流水。
          </p>
          <TextField
            label={`盘点后数量（${prop(entity!, "unit")}）`}
            type="number"
            min={0}
            step={1}
            value={counted}
            onChange={setCounted}
          />
          <TextField label="盘点说明" value={notes} onChange={setNotes} maxLength={300} required={false} />
        </>
      )}
    </Modal>
  );
}
