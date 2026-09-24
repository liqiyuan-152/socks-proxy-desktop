export type ProxyMode = "rules" | "global" | "direct";

export const proxyModes: Record<ProxyMode, { label: string; description: string; rule: string }> = {
  rules: {
    label: "规则代理",
    description: "根据分流规则决定流量走向",
    rule: "公司内网 / 常用站点",
  },
  global: {
    label: "全局代理",
    description: "遵循 Windows 系统代理的应用流量经当前代理转发",
    rule: "系统代理应用流量经代理",
  },
  direct: {
    label: "全局直连",
    description: "恢复应用管理的系统代理设置",
    rule: "恢复系统代理",
  },
};
