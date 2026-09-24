export type ParsedProxyLink = {
  name: string;
  protocol: "socks5" | "http";
  server: string;
  port: string;
  authentication: boolean;
  username: string;
  password: string;
};

export function parseProxyLink(input: string): ParsedProxyLink {
  let link = input.trim();
  if (link.startsWith("【") && link.endsWith("】")) {
    link = link.slice(1, -1).trim();
  }

  if (/\s/.test(link)) throw new Error("代理链接不能包含空白字符");

  const scheme = /^([a-z][a-z\d+.-]*):\/\//i.exec(link)?.[1]?.toLowerCase();
  if (scheme !== "socks5" && scheme !== "http") {
    throw new Error("仅支持 SOCKS5 或 HTTP 代理链接");
  }

  // Some copied links escape the separator between credentials and host.
  const separator = link.lastIndexOf("@");
  if (separator > 0 && link[separator - 1] === "\\") {
    link = link.slice(0, separator - 1) + link.slice(separator);
  }
  if (separator > 0) {
    link = link.slice(0, separator).replaceAll("\\", "%5C") + link.slice(separator);
  }

  const authority = link.slice(link.indexOf("://") + 3).split(/[/?#]/, 1)[0];
  const rawPort = /:(\d+)$/.exec(authority)?.[1];
  if (rawPort && Number(rawPort) > 65535) {
    throw new Error("端口必须在 1 到 65535 之间");
  }

  let url: URL;
  try {
    url = new URL(link);
  } catch {
    if (/^\w+:\/\/(?:[^@/]*@)?:\d+\/?$/.test(link)) {
      throw new Error("代理链接缺少服务器地址");
    }
    throw new Error("代理链接格式无效");
  }

  if (!url.hostname || (url.pathname !== "" && url.pathname !== "/") || url.search || url.hash) {
    throw new Error("代理链接格式无效或缺少服务器地址");
  }
  if (!rawPort || Number(rawPort) < 1) {
    throw new Error("端口必须在 1 到 65535 之间");
  }

  let username: string;
  let password: string;
  try {
    username = decodeURIComponent(url.username);
    password = decodeURIComponent(url.password);
  } catch {
    throw new Error("用户名或密码的编码无效");
  }
  if ((username === "") !== (password === "")) {
    throw new Error("启用认证时须同时提供用户名和密码");
  }

  const server = url.hostname.replace(/^\[|\]$/g, "");
  const port = String(Number(rawPort));
  return {
    name: `${url.hostname}:${port}`,
    protocol: scheme,
    server,
    port,
    authentication: username !== "",
    username,
    password,
  };
}
