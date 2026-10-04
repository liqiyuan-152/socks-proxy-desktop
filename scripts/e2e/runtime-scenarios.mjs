import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import { setTimeout } from "node:timers/promises";
import { inspectNative, probeTunnel } from "./native-runtime-tools.mjs";
import { xpathText } from "./webdriver.mjs";
import { recoverThroughUi } from "./scenarios.mjs";

/** 实际页面、进程和用户代理验收；复用此套件的独立档案，不改业务数据文件。 */
export async function runRuntimeScenarios(
  driver,
  { port, reportDirectory, onCase, onObservation, durationSeconds },
) {
  const seedName = "E2E 原生代理";
  let profileName = seedName;
  const navigate = (path) => driver.click("css selector", `a[href="${path}"]`);
  const button = (name) => driver.click("xpath", xpathText("button", name));
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
  const select = (name) =>
    driver.click("xpath", `//*[@role='tab' and normalize-space(.)='${name}']`);
  const healthy = async (name) => {
    await pair("运行阶段", "运行中");
    await pair("会话健康", "健康");
    await pair("已应用模式", name);
    await contains("footer", name);
    await contains("footer", profileName);
  };
  const mode = async (name) => {
    await navigate("/");
    await select(name);
    await healthy(name);
  };
  const inspect = () => inspectNative(reportDirectory);
  const edit = async (name) => {
    await navigate("/proxies");
    await driver.click("css selector", `button[aria-label="编辑${profileName}"]`);
    await driver.fill("#proxy-name", name);
    await driver.click("css selector", '[role="dialog"] button[type="submit"]');
    await contains("tbody", name);
    profileName = name;
  };

  await navigate("/proxies");
  const rows = await driver.command("POST", "/elements", {
    using: "css selector",
    value: "tbody tr",
  });
  assert.equal(rows.length, 1, "只复用此隔离套件的一条代理");
  await contains("tbody", seedName);
  assert.equal((await inspect()).proxy_restored, true, "初始网络必须与本次运行前一致");
  await driver.click("css selector", `button[aria-label="编辑${seedName}"]`);
  await driver.fill("#proxy-port", "0");
  await driver.click("css selector", '[role="dialog"] button[type="submit"]');
  await contains('[role="dialog"]', "端口");
  await driver.wait("无效端口被拒绝", async () => {
    await driver.find("css selector", '[role="dialog"] [role="alert"]');
    return true;
  });
  assert.equal(await driver.attribute("#proxy-port", "value"), "0");
  assert.equal(await driver.attribute("#proxy-name", "value"), seedName);
  await onCase("原生无效配置反馈与保留输入");
  await driver.fill("#proxy-port", String(port));
  await driver.click("css selector", '[role="dialog"] button[type="submit"]');
  await contains("tbody", seedName);
  await mode("规则代理");

  // 不等待中间状态，实际用户快速选择最后请求；只通过页面检查最终一致性。
  await select("全局代理");
  await select("全局直连");
  await select("规则代理");
  await navigate("/logs");
  await navigate("/");
  await healthy("规则代理");
  const beforeEdit = await inspect();
  await edit("E2E 状态同步验收");
  await navigate("/");
  await healthy("规则代理");
  assert.deepEqual((await inspect()).core_pids, beforeEdit.core_pids, "元数据保存不重启健康内核");
  await edit(seedName);
  await navigate("/");
  await healthy("规则代理");
  await onCase("原生快速模式选择、页面切换与配置保存后状态一致");

  const termination = await inspectNative(reportDirectory, true);
  const detectedAt = performance.now();
  await pair("运行阶段", "异常");
  await pair("会话健康", "已退出");
  await pair("已应用模式", "未应用");
  await pair("覆盖范围", "未接管系统代理");
  await contains('[aria-label="运行时反馈"]', "重试模式切换");
  const recovered = await inspect();
  assert.equal(recovered.proxy_restored, true, "崩溃后五字段网络设置自动恢复");
  assert.deepEqual(recovered.core_pids, []);
  assert.equal(recovered.runtime_directories, 0);
  await onObservation({
    crash: { ...termination, detected_in_ui_ms: performance.now() - detectedAt, recovered },
  });
  await onCase("原生内核崩溃提示与自动恢复真实用户网络");
  await setTimeout(3000);
  assert.deepEqual((await inspect()).core_pids, [], "等待用户重试期间不自动重启");
  await button("重试模式切换");
  await healthy("规则代理");
  const retried = await inspect();
  assert.equal(retried.core_pids.length, 1);
  assert.notEqual(retried.core_pids[0], termination.terminated_core_pid);
  await probeTunnel(retried.proxy_port);
  await onObservation({ retry: retried });
  await onCase("原生手动重试创建健康的新内核会话");

  const started = performance.now();
  let probes = 0;
  let transitions = 0;
  let metadataSaves = 0;
  let nextSample = 0;
  let nextSwitch = 30_000;
  let nextEdit = 60_000;
  let activeMode = "规则代理";
  let current = retried;
  const samples = [];
  // 每轮真实 UI/隧道检查依赖当前会话，轮询不可并发。
  /* oxlint-disable no-await-in-loop */
  while (performance.now() - started < durationSeconds * 1000) {
    const elapsed = performance.now() - started;
    if (elapsed >= nextSwitch) {
      activeMode = activeMode === "规则代理" ? "全局代理" : "规则代理";
      await mode(activeMode);
      current = await inspect();
      transitions++;
      nextSwitch += 30_000;
    }
    if (elapsed >= nextEdit) {
      const previous = current.core_pids;
      await edit(profileName === seedName ? "E2E 长时元数据" : seedName);
      await navigate("/");
      await healthy(activeMode);
      current = await inspect();
      assert.deepEqual(current.core_pids, previous);
      metadataSaves++;
      nextEdit += 60_000;
    }
    await probeTunnel(current.proxy_port);
    probes++;
    if (elapsed >= nextSample) {
      await navigate(elapsed % 60_000 < 30_000 ? "/rules" : "/logs");
      await navigate("/");
      await healthy(activeMode);
      current = await inspect();
      assert.equal(current.core_pids.length, 1);
      assert.equal(current.proxy_restored, false);
      assert.equal(current.runtime_directories, 1);
      assert.ok(current.processes.some((item) => item.role === "webview_or_child"));
      samples.push({ elapsed_seconds: (performance.now() - started) / 1000, ...current });
      await onObservation({
        soak: {
          requested_seconds: durationSeconds,
          elapsed_seconds: (performance.now() - started) / 1000,
          probes,
          transitions,
          metadata_saves: metadataSaves,
          samples,
        },
      });
      nextSample += 10_000;
    }
    await setTimeout(1000);
  }
  /* oxlint-enable no-await-in-loop */
  if (profileName !== seedName) await edit(seedName);
  await navigate("/");
  await healthy(activeMode);
  await onObservation({
    soak: {
      requested_seconds: durationSeconds,
      elapsed_seconds: (performance.now() - started) / 1000,
      probes,
      transitions,
      metadata_saves: metadataSaves,
      samples,
      completed: true,
    },
  });
  await onCase("原生应用与 WebView 持续运行、双向隧道、模式/元数据切换及资源采样");
  await recoverThroughUi(driver);
  await navigate("/");
  await pair("运行阶段", "未运行");
  await pair("覆盖范围", "未接管系统代理");
  const stopped = await inspect();
  assert.equal(stopped.proxy_restored, true);
  assert.equal(stopped.runtime_directories, 0);
  assert.deepEqual(stopped.core_pids, []);
  await onObservation({ stopped });
  await onCase("4.5.5 从原生运行界面恢复网络并完整还原用户系统代理");
}
