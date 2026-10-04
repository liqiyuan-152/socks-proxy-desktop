import assert from "node:assert/strict";
import { readFile, access } from "node:fs/promises";
import { xpathText } from "./webdriver.mjs";

const proxyName = "E2E 原生代理";
const ruleName = "E2E 域名直连";
const routeSection = '[aria-labelledby="route-test-title"]';

/** 原生保存窗口保留人工检查点；不模拟业务 IPC 或替换 dialog 插件。 */
export async function exportWithNativeDialog(driver, destination) {
  try {
    await access(destination);
    throw new Error(`导出目标已存在，拒绝将旧文件计为新结果：${destination}`);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  console.log(`请在接下来打开的原生保存窗口保存到：${destination}`);
  await driver.click("xpath", xpathText("button", "导出配置"));
  await driver.wait(
    "人工完成原生保存",
    async () => {
      try {
        JSON.parse(await readFile(destination, "utf8"));
        return true;
      } catch (error) {
        if (error.code === "ENOENT" || error instanceof SyntaxError) return false;
        throw error;
      }
    },
    900_000,
  );
  await driver.wait("界面确认保存成功", async () =>
    (await driver.text("body")).includes("已保存不含密码的配置"),
  );
  const text = await readFile(destination, "utf8");
  const configuration = JSON.parse(text);
  assert.equal(configuration.schema_version, 2);
  assert.ok(!/"password"\s*:/u.test(text), "导出不应包含密码字段");
  return configuration;
}

/** 重跑只清理此套件创建的一条代理与规则，拒绝删除其他档案。 */
export async function resetFixture(driver) {
  // 清理是同一个用户界面的顺序操作，不可并发。
  /* oxlint-disable no-await-in-loop */
  await recoverThroughUi(driver);
  for (const [path, name] of [
    ["/rules", ruleName],
    ["/proxies", proxyName],
  ]) {
    await driver.click("css selector", `a[href="${path}"]`);
    const rows = await driver.command("POST", "/elements", {
      using: "css selector",
      value: "tbody tr",
    });
    assert.equal(rows.length, 1, "重跑清理只接受本套件的一条记录");
    assert.ok((await driver.text("tbody")).includes(name), "发现非本套件档案，拒绝清理");
    if (path === "/rules") {
      const toggle = '[role="switch"][aria-label="国内直连"]';
      if ((await driver.attribute(toggle, "aria-checked")) === "true") {
        await driver.click("css selector", toggle);
        await driver.wait(
          "国内直连关闭",
          async () => (await driver.attribute(toggle, "aria-checked")) === "false",
        );
      }
    } else {
      await driver.click("xpath", xpathText("button", "取消默认代理"));
    }
    await driver.click("css selector", `button[aria-label="删除${name}"]`);
    await driver.click("xpath", xpathText("button", "确认删除"));
    await driver.wait("套件记录已删除", async () => !(await driver.text("tbody")).includes(name));
  }
  /* oxlint-enable no-await-in-loop */
}

export async function runScenarios(driver, { port, reportDirectory, onCase, reset }) {
  if (reset) await resetFixture(driver);
  const button = (name) => driver.click("xpath", xpathText("button", name));
  const navigate = (path) => driver.click("css selector", `a[href="${path}"]`);
  const contains = (selector, text) =>
    driver.wait(text, async () => (await driver.text(selector)).includes(text));
  const pair = (label, text) =>
    driver.wait(`${label}=${text}`, async () => {
      await driver.find(
        "xpath",
        `//span[normalize-space(.)='${label}']/following-sibling::span[1][normalize-space(.)='${text}']`,
      );
      return true;
    });
  const mode = async (name, phase) => {
    await navigate("/");
    await driver.click("xpath", `//*[@role='tab' and normalize-space(.)='${name}']`);
    await pair("运行阶段", phase);
    await pair("已应用模式", name);
  };
  const caseRun = async (name, action) => {
    await action();
    await onCase(name);
  };

  await caseRun("4.5.2 添加代理并启动规则模式", async () => {
    await navigate("/proxies");
    await contains("tbody", "暂无匹配的代理档案");
    await button("添加代理");
    await driver.fill("#proxy-name", proxyName);
    await driver.fill("#proxy-server", "127.0.0.1");
    await driver.fill("#proxy-port", String(port));
    await driver.click("css selector", "#proxy-protocol");
    await driver.click("xpath", "//*[@role='option' and normalize-space(.)='HTTP']");
    await driver.click("css selector", '[role="dialog"] button[type="submit"]');
    await contains("tbody", proxyName);
    await driver.click("css selector", `button[aria-label="设${proxyName}为默认代理"]`);
    await mode("规则代理", "运行中");
    await pair("会话健康", "健康");
  });

  await caseRun("4.5.3 路由规则和国内直连", async () => {
    await navigate("/rules");
    await contains("tbody", "暂无分流规则");
    await button("添加规则");
    await driver.fill("#rule-name", ruleName);
    await driver.fill("#rule-target", "e2e.invalid");
    await driver.click("xpath", "//*[@role='dialog']//button[normalize-space(.)='直连']");
    await driver.click("css selector", '[role="dialog"] button[type="submit"]');
    await contains("tbody", ruleName);
    await driver.click("css selector", '[role="switch"][aria-label="国内直连"]');
    await driver.wait(
      "国内直连已启用",
      async () =>
        (await driver.attribute('[role="switch"][aria-label="国内直连"]', "aria-checked")) ===
        "true",
    );
    // 每轮修改同一个表单并核对结果，必须顺序执行。
    /* oxlint-disable no-await-in-loop */
    for (const [target, expected] of [
      ["e2e.invalid", ruleName],
      ["127.0.0.1", "私有或本地 IP"],
    ]) {
      await driver.fill(`${routeSection} input:not([type="number"])`, target);
      await driver.click("css selector", `${routeSection} button[type="submit"]`);
      await contains(routeSection, expected);
      await contains(routeSection, "直连");
    }
    /* oxlint-enable no-await-in-loop */
    await mode("全局代理", "运行中");
    await mode("全局直连", "未运行");
  });

  await caseRun("4.5.4 原生保存与配置导入往返（人工保存检查点）", async () => {
    await navigate("/settings");
    const before = await exportWithNativeDialog(driver, `${reportDirectory}/before.json`);
    await navigate("/proxies");
    await driver.click("css selector", `button[aria-label="编辑${proxyName}"]`);
    await driver.fill("#proxy-name", "E2E 导入前临时修改");
    await driver.click("css selector", '[role="dialog"] button[type="submit"]');
    await contains("tbody", "E2E 导入前临时修改");
    await navigate("/settings");
    await driver.fill('input[type="file"]', `${reportDirectory}/before.json`, false);
    await button("确认导入");
    await contains("body", "配置已导入");
    const after = await exportWithNativeDialog(driver, `${reportDirectory}/after.json`);
    assert.deepEqual(after, before, "原生往返后完整配置应相等");
    await navigate("/proxies");
    await contains("tbody", proxyName);
  });

  await caseRun("4.5.5 用户网络恢复", async () => {
    await mode("规则代理", "运行中");
    await navigate("/settings");
    await button("恢复网络设置");
    await button("确认恢复");
    await contains("body", "网络恢复检查完成");
    await navigate("/");
    await pair("运行阶段", "未运行");
    await pair("覆盖范围", "未接管系统代理");
  });
}

/** 失败时先经真实界面恢复；失败证据保留，不直接写注册表。 */
export async function recoverThroughUi(driver) {
  await driver.click("css selector", 'a[href="/settings"]');
  await driver.click("xpath", xpathText("button", "恢复网络设置"));
  await driver.click("xpath", xpathText("button", "确认恢复"));
  await driver.wait("清理时网络恢复", async () =>
    (await driver.text("body")).includes("网络恢复检查完成"),
  );
}
