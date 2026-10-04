import assert from "node:assert/strict";
import { createServer } from "node:http";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { DesktopDriver, WebDriverError, xpathText } from "./e2e/webdriver.mjs";
import { exportWithNativeDialog } from "./e2e/scenarios.mjs";
import { resetFixture } from "./e2e/scenarios.mjs";
import { probeTunnel } from "./e2e/native-runtime-tools.mjs";
import { runWindows } from "./e2e/run-windows.mjs";

test("原生 UI 入口在非 Windows 或未明确启用时拒绝启动", async () => {
  await Promise.all(
    [
      { platform: "darwin", allowUi: true },
      { platform: "win32", allowUi: false },
    ].map((options) => assert.rejects(runWindows(options), /仅 Windows/u)),
  );
  assert.throws(() => new DesktopDriver("https://example.com"), /loopback/u);
  assert.throws(() => xpathText("button", "name'"), /单引号/u);
  assert.equal(xpathText("button", "保存"), "//button[normalize-space(.)='保存']");
  await assert.rejects(
    runWindows({ platform: "win32", allowUi: true, suite: "unknown" }),
    /未知原生套件/u,
  );
  await assert.rejects(
    runWindows({ platform: "win32", allowUi: true, durationSeconds: 0 }),
    /持续时间/u,
  );
});

test("原生重跑拒绝删除非本套件的档案", async () => {
  const clicks = [];
  const driver = {
    click: async (using, value) => clicks.push([using, value]),
    wait: async (_label, check) => assert.equal(await check(), true),
    text: async (selector) => (selector === "body" ? "网络恢复检查完成" : "用户自己的规则"),
    command: async () => [{ element: "unrelated" }],
  };
  await assert.rejects(resetFixture(driver), /非本套件档案/u);
  assert.ok(!clicks.some(([, value]) => value.includes("删除")));
});

test("隧道探测验证实际 CONNECT 与 GET 双向响应，并拒绝错误状态", async (context) => {
  let connects = 0;
  const server = createServer();
  server.on("connect", (_request, socket) => {
    if (++connects === 2) {
      socket.end("HTTP/1.1 502 Failed\r\n\r\n");
      return;
    }
    socket.write("HTTP/1.1 200 Connection Established\r\n\r\n");
    socket.once("data", (data) => {
      assert.ok(data.toString().startsWith("GET / HTTP/1.1"));
      socket.end("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nE2E");
    });
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  context.after(() => new Promise((done) => server.close(done)));
  await probeTunnel(server.address().port);
  await assert.rejects(probeTunnel(server.address().port), /CONNECT 失败/u);
  await assert.rejects(probeTunnel(0), /缺少本次内核代理端口/u);
});

test("W3C 会话：等候延迟出现的元素，输入/读取并正常关闭", async (context) => {
  let attempts = 0;
  let saved = "";
  let deleted = false;
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = chunks.length ? JSON.parse(Buffer.concat(chunks).toString()) : null;
    let value;
    switch (request.url) {
      case "/session":
        assert.equal(body.capabilities.alwaysMatch["tauri:options"].application, "fixture.exe");
        value = { sessionId: "fixture/session" };
        break;
      case "/session/fixture%2Fsession/element":
        if (++attempts === 1) {
          response.statusCode = 404;
          value = { error: "no such element", message: "not mounted yet" };
        } else value = { "element-6066-11e4-a52e-4f735466cecf": "element/id" };
        break;
      case "/session/fixture%2Fsession/element/element%2Fid/enabled":
      case "/session/fixture%2Fsession/element/element%2Fid/displayed":
        value = true;
        break;
      case "/session/fixture%2Fsession/element/element%2Fid/value":
        saved = body.text;
        value = null;
        break;
      case "/session/fixture%2Fsession/element/element%2Fid/text":
        value = saved;
        break;
      case "/session/fixture%2Fsession/element/element%2Fid/attribute/aria-checked":
        value = "true";
        break;
      case "/session/fixture%2Fsession/element/element%2Fid/clear":
      case "/session/fixture%2Fsession/element/element%2Fid/click":
        value = null;
        break;
      case "/session/fixture%2Fsession":
        assert.equal(request.method, "DELETE");
        deleted = true;
        value = null;
        break;
      default:
        response.statusCode = 404;
        value = { error: "unknown command", message: "unsupported fixture request" };
    }
    response.setHeader("Content-Type", "application/json");
    response.end(JSON.stringify({ value }));
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  context.after(() => new Promise((done) => server.close(done)));
  const driver = new DesktopDriver(`http://127.0.0.1:${server.address().port}`);
  await assert.rejects(driver.text("body"), /未启动/u);
  await driver.start("fixture.exe");
  await assert.rejects(driver.start("fixture.exe"), /已有/u);
  await driver.click("css selector", "button");
  await driver.fill("input", "原生配置");
  assert.equal(await driver.text("body"), "原生配置");
  assert.equal(await driver.attribute("input", "aria-checked"), "true");
  await driver.stop();
  await driver.stop();
  assert.equal(deleted, true);
});

test("轮询只重试元素缺失/陈旧；业务失败直接返回，等待有期限", async () => {
  const driver = new DesktopDriver();
  let calls = 0;
  await driver.wait("delayed", async () => {
    if (++calls === 1) throw new WebDriverError("stale element reference", "rerender");
    return true;
  });
  await assert.rejects(
    driver.wait("failure", () => {
      throw new WebDriverError("invalid session id", "gone");
    }),
    /gone/u,
  );
  await assert.rejects(
    driver.wait("deadline", () => false, 1),
    /等待超时/u,
  );
});

test("人工原生保存检查点拒绝旧文件和密码泄漏；仅新文件加界面确认才通过", async (context) => {
  const directory = await mkdtemp(join(tmpdir(), "e2e-save-fixture-"));
  context.after(() => rm(directory, { recursive: true, force: true }));
  const existing = join(directory, "existing.json");
  await writeFile(existing, "{}");
  await assert.rejects(exportWithNativeDialog({}, existing), /目标已存在/u);
  const configuration = { schema_version: 2, profiles: [], rules: [] };
  const destination = join(directory, "new.json");
  const driver = {
    click: () => writeFile(destination, JSON.stringify(configuration)),
    wait: async (_label, check) => assert.equal(await check(), true),
    text: async () => "已保存不含密码的配置",
  };
  assert.deepEqual(await exportWithNativeDialog(driver, destination), configuration);
  assert.deepEqual(JSON.parse(await readFile(destination, "utf8")), configuration);
  configuration.password = "synthetic-test-only";
  const unsafe = join(directory, "unsafe.json");
  driver.click = () => writeFile(unsafe, JSON.stringify(configuration));
  await assert.rejects(exportWithNativeDialog(driver, unsafe), /密码字段/u);
});
