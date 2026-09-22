export type ProxyMode = "rules" | "global" | "direct";

export const proxyModes: Record<ProxyMode, { label: string; description: string; rule: string }> = {
  rules: {
    label: "规则代理",
    description: "根据分流规则决定流量走向",
    rule: "公司内网 / 常用站点",
  },
  global: {
    label: "全局代理",
    description: "所有网络流量均通过当前 SOCKS5 代理",
    rule: "全部流量经代理",
  },
  direct: {
    label: "全局直连",
    description: "所有网络流量均绕过代理直接连接",
    rule: "全部流量直连",
  },
};
