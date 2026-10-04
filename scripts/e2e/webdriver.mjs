import { setTimeout } from "node:timers/promises";

const elementKey = "element-6066-11e4-a52e-4f735466cecf";

export class WebDriverError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

/** W3C 公共协议客户端；不注入 JavaScript，不调用 Tauri 业务命令。 */
export class DesktopDriver {
  constructor(endpoint = "http://127.0.0.1:4445") {
    const url = new URL(endpoint);
    if (url.hostname !== "127.0.0.1" || url.protocol !== "http:") {
      throw new Error("E2E 驱动必须使用本机 loopback HTTP 地址");
    }
    this.endpoint = url.origin;
    this.session = null;
  }

  async request(method, path, body) {
    const response = await fetch(`${this.endpoint}${path}`, {
      method,
      headers: { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(30_000),
    });
    const result = await response.json();
    if (!response.ok || result.value?.error) {
      throw new WebDriverError(result.value?.error ?? "transport error", result.value?.message);
    }
    return result.value;
  }

  async start(application) {
    if (this.session) throw new Error("已有 E2E 会话");
    const result = await this.request("POST", "/session", {
      capabilities: { alwaysMatch: { "tauri:options": { application } } },
    });
    if (!result.sessionId) throw new Error("驱动未返回 sessionId");
    this.session = result.sessionId;
  }

  async command(method, path, body) {
    if (!this.session) throw new Error("E2E 会话未启动");
    return this.request(method, `/session/${encodeURIComponent(this.session)}${path}`, body);
  }

  async find(using, value) {
    const found = await this.command("POST", "/element", { using, value });
    if (!found[elementKey]) throw new Error("驱动未返回元素引用");
    return encodeURIComponent(found[elementKey]);
  }

  async wait(label, check, timeout = 15_000) {
    const deadline = Date.now() + timeout;
    // 驱动轮询依赖上一轮结果，不能并发重复操作同一 UI。
    /* oxlint-disable no-await-in-loop */
    while (Date.now() < deadline) {
      try {
        if (await check()) return;
      } catch (error) {
        if (
          !(error instanceof WebDriverError) ||
          !["no such element", "stale element reference"].includes(error.code)
        ) {
          throw error;
        }
      }
      await setTimeout(100);
    }
    /* oxlint-enable no-await-in-loop */
    throw new Error(`等待超时：${label}`);
  }

  async click(using, value) {
    await this.wait(value, async () => {
      const id = await this.find(using, value);
      if (!(await this.command("GET", `/element/${id}/enabled`))) return false;
      if (!(await this.command("GET", `/element/${id}/displayed`))) return false;
      await this.command("POST", `/element/${id}/click`, {});
      return true;
    });
  }

  async fill(selector, text, clear = true) {
    const id = await this.find("css selector", selector);
    if (clear) await this.command("POST", `/element/${id}/clear`, {});
    await this.command("POST", `/element/${id}/value`, { text });
  }

  async text(selector) {
    const id = await this.find("css selector", selector);
    return this.command("GET", `/element/${id}/text`);
  }

  async attribute(selector, name) {
    const id = await this.find("css selector", selector);
    return this.command("GET", `/element/${id}/attribute/${encodeURIComponent(name)}`);
  }

  async stop() {
    if (!this.session) return;
    await this.command("DELETE", "");
    this.session = null;
  }
}

export function xpathText(tag, text) {
  if (text.includes("'")) throw new Error("测试控件名称不允许单引号");
  return `//${tag}[normalize-space(.)='${text}']`;
}
