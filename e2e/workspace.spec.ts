import { readFile } from "node:fs/promises";
import { test, expect, type Page } from "@playwright/test";

// The second test logs in to the account the first one creates.
test.describe.configure({ mode: "serial" });
const password = "test-ui-owner-password-1234";
async function navigate(page: Page, label: string) {
  if ((page.viewportSize()?.width || 1440) <= 800)
    await page.getByRole("button", { name: "打开导航", exact: true }).click();
  await page
    .getByRole("navigation", { name: "主导航" })
    .getByRole("button", { name: label, exact: label !== "决策中心" })
    .click();
  await expect(
    page.getByRole("heading", { name: label, exact: true, level: 1 }),
  ).toBeVisible();
}
async function unlock(page: Page) {
  await page.getByRole("button", { name: "本人解锁", exact: true }).click();
  await page.getByRole("dialog").getByLabel("账户口令").fill(password);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "解锁", exact: true })
    .click();
  await expect(page.getByText("本人修改 · 按授权只读", { exact: true })).toBeVisible();
}
async function confirm(page: Page) {
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("账户口令").fill(password);
  await dialog.getByRole("checkbox").last().check();
}

test("owner-controlled three-world workflow, persistence and responsive screens", async ({
  page, browser,
}, testInfo) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "建立你的资源空间" }),
  ).toBeVisible();
  await page.getByLabel("设置账户口令").fill(password);
  await page.getByLabel("再次输入口令").fill(password);
  await page.getByRole("button", { name: "创建本地账户" }).click();
  await expect(
    page.getByRole("heading", { name: "工作台", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "添加业务演示数据" }).click();
  await expect(
    page.getByText("整理本周学习资料", { exact: true }),
  ).toBeVisible();

  await navigate(page, "思维体系");
  await unlock(page);
  await page.getByRole("button", { name: "新建条目" }).click();
  let dialog = page.getByRole("dialog");
  await dialog.getByLabel("标题", { exact: true }).fill("PRIVATE-UI-自主学习");
  await dialog
    .getByLabel("我的判断与记录")
    .fill("PRIVATE-UI-每天保留独立阅读和思考的时间。");
  await dialog.getByLabel("存在", { exact: true }).check();
  await confirm(page);
  await dialog.getByRole("button", { name: "本人确认并保存" }).click();
  await expect(
    page.getByRole("heading", { name: "PRIVATE-UI-自主学习" }),
  ).toBeVisible();
  const recipientContext = await browser.newContext();
  const recipient = await recipientContext.newPage();
  await recipient.goto("http://127.0.0.1:4318/shared");
  await expect(recipient.getByLabel("本设备公钥")).not.toHaveValue("");
  const recipientKey = await recipient.getByLabel("本设备公钥").inputValue();
  await recipientContext.close();
  await page.bringToFront();
  if (await page.getByRole("button", { name: "本人解锁", exact: true }).isVisible()) await unlock(page);
  const permissions = page.locator(".permissions-panel");
  await permissions.locator("summary").click();
  await permissions.getByLabel("接收者名称").fill("只读测试设备");
  await permissions.getByLabel("接收者设备公钥").fill(recipientKey);
  await permissions.getByRole("checkbox", { name: "PRIVATE-UI-自主学习" }).check();
  await permissions.getByLabel("本人确认口令").fill(password);
  await permissions.getByRole("button", { name: "确认只读授权" }).click();
  await expect(permissions.locator(".grant-row")).toContainText("只读测试设备");
  await permissions.getByRole("button", { name: "撤销授权" }).click();
  await expect(permissions.locator(".grant-row")).toContainText("已撤销");
  await permissions.locator("summary").click();
  await page.getByRole("button", { name: "发布执行方向" }).click();
  dialog = page.getByRole("dialog");
  await expect(dialog.getByLabel("标题", { exact: true })).toHaveValue("");
  await expect(dialog.getByLabel("允许助理读取的执行要求")).toHaveValue("");
  await dialog.getByLabel("标题", { exact: true }).fill("本周学习安排");
  await dialog
    .getByLabel("允许助理读取的执行要求")
    .fill("收集资料，并安排一次自主阅读。");
  await confirm(page);
  await dialog.getByRole("button", { name: "确认发布" }).click();
  await expect(dialog).not.toBeVisible();

  await navigate(page, "数字空间");
  const locked = await page.request.get("/api/vault");
  expect(locked.status()).toBe(403); // A cookie alone cannot read the vault.
  await page.getByLabel("消息内容").fill("整理本周的阅读笔记");
  await page.getByRole("button", { name: "发送", exact: true }).click();
  await expect(
    page.locator(".message-meta").filter({ hasText: "离线助理 · 规则分析" }),
  ).toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("digital-desktop.png"),
    fullPage: true,
  });

  await navigate(page, "决策中心");
  const proposal = page.locator(".proposal").filter({
    has: page.getByRole("heading", { name: "跟进：整理本周的阅读笔记" }),
  });
  await proposal.getByRole("button", { name: "审阅并采纳" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("标题", { exact: true }).fill("完成一次自主阅读");
  await dialog
    .getByLabel("关联执行方向")
    .selectOption({ label: "本周学习安排" });
  await dialog.getByRole("button", { name: "确认创建任务" }).click();
  await expect(dialog).not.toBeVisible();
  await navigate(page, "工作台");
  await expect(
    page.getByText("完成一次自主阅读", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "完成：完成一次自主阅读", exact: true })
    .click();
  await expect(page.getByLabel("完成一次自主阅读的状态")).toHaveValue("done");

  await navigate(page, "物理资源");
  await page
    .getByRole("button", { name: "滤芯 / 家用净水器入库", exact: true })
    .click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("数量（个）").fill("2");
  await dialog.getByLabel("变动原因").fill("日常补货");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await page
    .getByRole("button", { name: "滤芯 / 家用净水器出库", exact: true })
    .click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("数量（个）").fill("99");
  await dialog.getByLabel("变动原因").fill("检查库存限制");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog.getByRole("alert")).toContainText("库存不足");
  await dialog.getByRole("button", { name: "取消", exact: true }).click();
  await page.getByRole("button", { name: "现金流水", exact: true }).click();
  await page.getByRole("button", { name: "记一笔", exact: true }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("标题", { exact: true }).fill("学习文具");
  await dialog.getByLabel("金额（元）").fill("12.34");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("学习文具", { exact: true })).toBeVisible();
  await expect(page.getByText("-¥12.34", { exact: true })).toBeVisible();

  await navigate(page, "本体关系");
  await page.screenshot({
    path: testInfo.outputPath("ontology-desktop.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: /实体与关系/ }).click();
  await page.getByRole("button", { name: "建立关系", exact: true }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("关系", { exact: true }).selectOption("uses");
  await dialog
    .getByLabel("起点实体")
    .selectOption({ label: "任务 · 完成一次自主阅读" });
  await dialog
    .getByLabel("终点实体")
    .selectOption({ label: "资源 · 工作笔记本电脑" });
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();

  await navigate(page, "思维体系");
  await unlock(page);
  await expect(
    page.getByRole("heading", { name: "PRIVATE-UI-自主学习" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "今日确认", exact: true }).click();
  await confirm(page);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "本人确认并保存" })
    .click();
  await expect(page.getByText(/版本 2 · 最近确认/)).toBeVisible();
  await page.getByRole("button", { name: "版本", exact: true }).click();
  await expect(
    page
      .getByRole("dialog")
      .getByRole("heading", { name: "v1 · PRIVATE-UI-自主学习" }),
  ).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  await page.getByRole("button", { name: "锁定思维库", exact: true }).click();
  await expect(
    page.getByText("PRIVATE-UI-每天保留独立阅读和思考的时间。", {
      exact: true,
    }),
  ).not.toBeVisible();

  await navigate(page, "活动记录");
  const downloadEvent = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出业务数据" }).click();
  const download = await downloadEvent;
  expect(download.suggestedFilename()).toMatch(/^prm-operations-/);
  const exported = await readFile((await download.path())!, "utf8");
  expect(exported).not.toContain("PRIVATE-UI");

  await navigate(page, "工作台");
  await expect(page.getByRole("status")).not.toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("dashboard-desktop.png"),
    fullPage: true,
  });
  for (const width of [390, 360, 768, 1440]) {
    await page.setViewportSize({ width, height: width < 800 ? 844 : 1000 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBeTruthy();
    if (width === 390) {
      await expect(
        page.getByRole("navigation", { name: "主导航" }),
      ).not.toBeVisible();
      await page.screenshot({
        path: testInfo.outputPath("dashboard-mobile.png"),
        fullPage: true,
      });
      for (const name of [
        "自我本体",
        "关系网络",
        "思维体系",
        "数字空间",
        "物理资源",
        "决策中心",
        "本体关系",
        "活动记录",
      ]) {
        await navigate(page, name);
        await expect(
          page.getByRole("navigation", { name: "主导航" }),
        ).not.toBeVisible();
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth <= window.innerWidth,
          ),
        ).toBeTruthy();
        await page.screenshot({
          path: testInfo.outputPath(`${name}-mobile.png`),
          fullPage: true,
        });
      }
      await navigate(page, "工作台");
    }
  }
  await navigate(page, "数字空间");
  await page.getByRole("button", { name: "执行方向", exact: true }).click();
  await page.getByRole("button", { name: /本周学习安排/ }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "撤回授权", exact: true })
    .click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await page.getByRole("button", { name: /本周学习安排/ }).click();
  await expect(
    page.getByRole("dialog").getByText("已撤回", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  await navigate(page, "工作台");
  await page.getByRole("button", { name: "退出账户", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "回到你的资源空间" }),
  ).toBeVisible();
  await page.getByLabel("账户口令").fill(password);
  await page.getByRole("button", { name: "进入工作区" }).click();
  await expect(
    page.getByText("完成一次自主阅读", { exact: true }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test("self ontology, relations, agent gateway and ERP purchasing", async ({
  page,
  playwright,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await page.getByLabel("账户口令").fill(password);
  await page.getByRole("button", { name: "进入工作区" }).click();
  await expect(
    page.getByRole("heading", { name: "工作台", exact: true }),
  ).toBeVisible();

  await navigate(page, "自我本体");
  await page.getByLabel("量表或指标组").selectOption("rses");
  await page.getByLabel("自尊总分（10–40）").fill("32");
  await page.getByRole("button", { name: "记录测评" }).click();
  await expect(page.getByText("RSES · 自尊总分", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /^自我认同/ }).click();
  await page.getByLabel("我是……").fill("我是一名终身学习者");
  await page.getByLabel("类别").selectOption("reflective");
  await page.getByLabel("作为核心设定提供给本地模型").check();
  await page.getByRole("button", { name: "记录自我认同" }).click();
  await expect(
    page.getByText("我是一名终身学习者", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Agent 可见预览" }).click();
  const preview = page.locator(".agent-preview");
  await expect(preview).toContainText("我是一名终身学习者");
  await expect(preview).not.toContainText("PRIVATE-UI");
  await expect(preview).not.toContainText("收集资料"); // The direction was revoked.

  await navigate(page, "关系网络");
  await page.getByRole("button", { name: "新建关系" }).click();
  let dialog = page.getByRole("dialog");
  await dialog.getByLabel("人物姓名").fill("李华");
  await dialog.getByLabel("关系类型").selectOption("friend");
  await dialog.getByLabel("亲密度（IOS 1–7）").selectOption("5");
  await dialog.getByLabel("联系节奏（天，0 为不提醒）").fill("14");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("heading", { name: "李华" })).toBeVisible();
  await page.getByRole("button", { name: "记录与李华的互动" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("方式").selectOption("call");
  await dialog.getByLabel("备注").fill("电话聊近况");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator(".relationship-card")).toContainText("上次");

  await navigate(page, "数字空间");
  await page.getByRole("button", { name: "Agent 接入", exact: true }).click();
  await page.getByLabel("连接名称").fill("测试 Agent");
  await page.getByLabel("接入方式").selectOption("http");
  await page.getByRole("button", { name: "签发令牌" }).click();
  const token = ((await page.locator(".token-box code").textContent()) || "").trim();
  expect(token).toMatch(/^prm_[0-9a-f]{64}$/);
  const agent = await playwright.request.newContext({
    baseURL: "http://127.0.0.1:4318",
  });
  const imported = await agent.post("/api/agent/import", {
    headers: { Authorization: `Bearer ${token}`, "X-PRM-Request": "1" },
    data: {
      scenes: ["work"],
      items: [
        { ref: "r1", kind: "resource", title: "会议室投影仪", attributes: { unit: "台", location: "会议室" } },
        { kind: "task", title: "周五前提交周报" },
      ],
    },
  });
  expect(imported.status()).toBe(200);
  expect((await imported.json()).proposals_created).toBe(2);
  const forbidden = await agent.get("/api/workspace", {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(forbidden.status()).toBe(403);
  await agent.dispose();

  await navigate(page, "决策中心");
  await page.getByLabel("建议来源").selectOption("import");
  const candidate = page.locator(".proposal").filter({
    has: page.getByRole("heading", { name: "会议室投影仪" }),
  });
  await expect(candidate).toContainText("纳入资源");
  await candidate.getByRole("button", { name: "审阅并采纳" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "确认纳入" }).click();
  await expect(dialog).not.toBeVisible();

  await navigate(page, "物理资源");
  await page.getByRole("button", { name: "供应商", exact: true }).click();
  await page.getByRole("button", { name: "新建供应商" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("供应商名称").fill("办公设备店");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await page.getByRole("button", { name: "采购订单", exact: true }).click();
  await page.getByRole("button", { name: "新建采购订单" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("订单名称").fill("会议室设备采购");
  await dialog.getByLabel("供应商").selectOption({ label: "办公设备店" });
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  const order = page.locator(".order-card").filter({
    has: page.getByRole("heading", { name: "会议室设备采购" }),
  });
  await order.getByRole("button", { name: "添加订单行" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByLabel("物料").selectOption({ label: "会议室投影仪（台）" });
  await dialog.getByLabel("数量").fill("2");
  await dialog.getByLabel("单价（元）").fill("1999.00");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await order.getByRole("button", { name: "确认下单" }).click();
  await expect(order).toContainText("已下单");
  await order.getByRole("button", { name: "收货" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(order).toContainText("已收货");
  await order.getByRole("button", { name: "付款" }).click();
  dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(order.locator(".order-totals")).toContainText("已付款 ¥3,998.00");
  await page.getByRole("button", { name: "现金流水", exact: true }).click();
  await expect(
    page.getByText("采购付款：会议室设备采购", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "资源台账", exact: true }).click();
  await expect(
    page.getByRole("row").filter({ hasText: "会议室投影仪" }).locator("td").nth(2),
  ).toContainText("2");

  await navigate(page, "本体关系");
  await expect(page.getByRole("heading", { name: "资源关系" })).toBeVisible();
  await page.getByLabel("人生场景").selectOption("work");
  await navigate(page, "工作台");
  await expect(page.locator(".scene-banner")).toContainText("当前场景：工作");
  await page.getByRole("button", { name: "显示全部场景" }).click();

  await page.setViewportSize({ width: 390, height: 844 });
  for (const [name, tab] of [
    ["自我本体", "Agent 可见预览"],
    ["关系网络", ""],
    ["数字空间", "Agent 接入"],
    ["物理资源", "采购订单"],
    ["决策中心", ""],
    ["本体关系", ""],
  ]) {
    await navigate(page, name);
    if (tab) await page.getByRole("button", { name: tab, exact: true }).click();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      name,
    ).toBeTruthy();
  }
  expect(errors).toEqual([]);
});
