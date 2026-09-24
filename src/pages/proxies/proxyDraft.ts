import type { ProxyProfile } from "@/lib/backend";
import type { ProxyDraft } from "./ProxyAuthenticationFields";

export function createProxyDraft(proxy?: ProxyProfile): ProxyDraft {
  return {
    name: proxy?.name ?? "",
    protocol: proxy?.protocol ?? "socks5",
    server: proxy?.host ?? "",
    port: proxy?.port?.toString() ?? "",
    authentication: proxy?.authentication_enabled ?? false,
    username: "",
    password: "",
  };
}
