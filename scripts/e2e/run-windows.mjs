import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { DesktopDriver } from "./webdriver.mjs";
import { runScenarios, recoverThroughUi } from "./scenarios.mjs";
import { runRuntimeScenarios } from "./runtime-scenarios.mjs";

export async function runWindows({
  application,
  reportDirectory,
  allowUi,
  resetFixture = false,
  suite = "journey",
  durationSeconds = 1800,
  platform = process.platform,
}) {
  if (platform !== "win32" || !allowUi) {
    throw new Error("仅 Windows 且明确传入 --run-ui 时执行真实原生 UI；预检不执行本脚本");
  }
  if (
    !["journey", "runtime"].includes(suite) ||
    !Number.isInteger(durationSeconds) ||
    durationSeconds < 1 ||
    durationSeconds > 86400
  ) {
    throw new Error("未知原生套件或无效持续时间");
  }
  if (resolve(application) !== resolve("src-tauri/target/debug/socks-proxy.exe")) {
    throw new Error("仅接受仓库内隔离 E2E 构建路径");
  }
  const binaryHash = createHash("sha256")
    .update(await readFile(application))
    .digest("hex");
  const preflight = JSON.parse(
    (await readFile(".e2e-tools/preflight.json", "utf8")).replace(/^\uFEFF/u, ""),
  );
  if (
    preflight.status !== "passed" ||
    preflight.application_identifier !== "com.socksproxy.desktop.architecture-e2e" ||
    preflight.application_sha256 !== binaryHash
  ) {
    throw new Error("当前二进制没有匹配的隔离 E2E 构建预检记录，请重新运行完整预检");
  }
  await mkdir(reportDirectory, { recursive: true });
  const report = {
    status: "running",
    platform: "Windows x64",
    native_ui: true,
    application_identifier: "com.socksproxy.desktop.architecture-e2e",
    application_sha256: binaryHash,
    suite,
    assisted_native_save: suite === "journey",
    cases: [],
    limitations: [
      "测试使用合成本地 HTTP 上游，不证明公网可达性",
      ...(suite === "journey"
        ? ["原生保存窗口由操作员完成", "进程崩溃自动恢复及长时 UI 稳定性另行验收"]
        : ["内核崩溃验证不代表桌面宿主崩溃", "资源采样观察进程树，不证明所有长期泄漏不存在"]),
    ],
  };
  const persist = () =>
    writeFile(`${reportDirectory}/result.json`, `${JSON.stringify(report, null, 2)}\n`);
  const sockets = new Set();
  const upstream = createServer((_request, response) => response.end("E2E local upstream"));
  upstream.on("connection", (socket) => {
    sockets.add(socket);
    socket.on("error", () => socket.destroy());
    socket.on("close", () => sockets.delete(socket));
  });
  upstream.on("connect", (_request, socket) => {
    socket.write("HTTP/1.1 200 Connection Established\r\n\r\n");
    socket.once("data", () =>
      socket.end("HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nE2E"),
    );
  });
  const driver = new DesktopDriver();
  await persist();
  try {
    await new Promise((resolveListen, reject) => {
      upstream.once("error", reject);
      upstream.listen(0, "127.0.0.1", resolveListen);
    });
    await driver.start(resolve(application));
    const scenarios = suite === "runtime" ? runRuntimeScenarios : runScenarios;
    await scenarios(driver, {
      port: upstream.address().port,
      reportDirectory: resolve(reportDirectory),
      reset: resetFixture,
      durationSeconds,
      onObservation: async (observation) => {
        Object.assign(report, observation);
        await persist();
      },
      onCase: async (name) => {
        const screenshot = await driver.command("GET", "/screenshot");
        const artifact = `case-${report.cases.length + 1}.png`;
        await writeFile(`${reportDirectory}/${artifact}`, Buffer.from(screenshot, "base64"));
        report.cases.push({ name, status: "passed", screenshot: artifact });
        await persist();
      },
    });
    report.status = "passed";
  } catch (error) {
    report.status = "failed";
    report.error = error.message;
  } finally {
    if (driver.session) {
      try {
        await recoverThroughUi(driver);
        report.cleanup = "network recovery through native UI completed";
      } catch (error) {
        report.status = "failed";
        report.cleanup_error = error.message;
      }
      try {
        await driver.stop();
      } catch (error) {
        report.status = "failed";
        report.session_cleanup_error = error.message;
      }
    }
    for (const socket of sockets) socket.destroy();
    if (upstream.listening) await new Promise((close) => upstream.close(close));
    await persist();
  }
  if (report.status !== "passed")
    throw new Error(`E2E 未通过，请检查 ${reportDirectory}/result.json`);
}

if (process.argv[1] && resolve(process.argv[1]) === import.meta.filename) {
  const args = process.argv.slice(2);
  const directoryIndex = args.indexOf("--report-directory");
  const suiteIndex = args.indexOf("--suite");
  const durationIndex = args.indexOf("--duration-seconds");
  try {
    if (directoryIndex < 0 || !args[directoryIndex + 1]) throw new Error("缺少 --report-directory");
    await runWindows({
      application: resolve("src-tauri/target/debug/socks-proxy.exe"),
      reportDirectory: args[directoryIndex + 1],
      allowUi: args.includes("--run-ui"),
      resetFixture: args.includes("--reset-fixture"),
      suite: suiteIndex < 0 ? "journey" : args[suiteIndex + 1],
      durationSeconds: durationIndex < 0 ? 1800 : Number(args[durationIndex + 1]),
    });
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
