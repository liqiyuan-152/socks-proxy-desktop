import { describe, expect, it } from "vitest";
import { parseProxyLink } from "./parseProxyLink";

describe("parseProxyLink", () => {
  it("parses the wrapped escaped SOCKS5 link", () => {
    expect(parseProxyLink("【socks5://repeatlink:abc0123456789\\@210.48.231.68:1080】")).toEqual({
      name: "210.48.231.68:1080",
      protocol: "socks5",
      server: "210.48.231.68",
      port: "1080",
      authentication: true,
      username: "repeatlink",
      password: "abc0123456789",
    });
  });

  it("decodes credentials without changing other backslashes", () => {
    expect(parseProxyLink("socks5://user:p%40ss%5Cword@proxy.example.com:1080")).toMatchObject({
      username: "user",
      password: "p@ss\\word",
      authentication: true,
    });
    expect(parseProxyLink("socks5://user:pa\\ss\\@proxy.example.com:1080").password).toBe("pa\\ss");
    expect(parseProxyLink("http://user:pa\\ss\\@proxy.example.com:8080").password).toBe("pa\\ss");
  });

  it("supports HTTP without credentials and unbracketed IPv6 server values", () => {
    expect(parseProxyLink("http://proxy.example.com:8080")).toEqual({
      name: "proxy.example.com:8080",
      protocol: "http",
      server: "proxy.example.com",
      port: "8080",
      authentication: false,
      username: "",
      password: "",
    });
    expect(parseProxyLink("http://proxy.example.com:80")).toMatchObject({
      name: "proxy.example.com:80",
      port: "80",
    });
    expect(parseProxyLink("socks5://u:p@[2001:db8::1]:1080")).toMatchObject({
      name: "[2001:db8::1]:1080",
      server: "2001:db8::1",
      username: "u",
      password: "p",
    });
  });

  it.each([
    ["https://host:1080", /仅支持/],
    ["socks5://:1080", /服务器地址/],
    ["socks5://host", /端口/],
    ["socks5://host:0", /端口/],
    ["socks5://host:65536", /端口/],
    ["http://host:1080/path", /格式无效/],
    ["socks5://user:password@host:1080?secret=1", /格式无效/],
    ["socks5://user:%ZZ@host:1080", /编码无效/],
    ["socks5://user@host:1080", /同时提供/],
  ])("rejects an invalid link without partial output: %s", (input, message) => {
    expect(() => parseProxyLink(input)).toThrow(message);
  });
});
