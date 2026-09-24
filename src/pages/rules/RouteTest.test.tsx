import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { RouteTest } from "./RouteTest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));

beforeEach(() => invoke.mockReset());

it("shows the predicted exit and user-rule explanation without implying a connection", async () => {
  invoke.mockResolvedValue({
    stage: "user_rule",
    action: "proxy",
    proxy_profile_id: "primary",
    proxy_name: "Primary",
    matched_rule_id: "first",
    matched_rule_name: "First",
    reason: "首条匹配的用户规则",
    data_date: "2026-09-23",
  });
  render(<RouteTest />);
  fireEvent.change(screen.getByRole("textbox", { name: "目标域名或 IP" }), {
    target: { value: "baidu.com" },
  });
  fireEvent.click(screen.getByRole("button", { name: "测试" }));
  await waitFor(() =>
    expect(screen.getByRole("status")).toHaveTextContent("代理：Primary · 规则：First"),
  );
  expect(screen.getByRole("status")).toHaveTextContent("命中阶段：用户规则");
  expect(screen.getByRole("status")).toHaveTextContent("不代表已建立连接或实际拨号地址");
  expect(invoke).toHaveBeenCalledWith("test_route", { target: "baidu.com", port: 443 });
});

it("explains that an unlisted domain uses the default exit without DNS inference", async () => {
  invoke.mockResolvedValue({
    stage: "final",
    action: "proxy",
    proxy_profile_id: "primary",
    proxy_name: "Primary",
    matched_rule_id: null,
    matched_rule_name: null,
    reason: "域名集外不按解析 IP 判断",
    data_date: "2026-09-23",
  });
  render(<RouteTest />);
  fireEvent.change(screen.getByRole("textbox", { name: "目标域名或 IP" }), {
    target: { value: "mixed.invalid" },
  });
  fireEvent.click(screen.getByRole("button", { name: "测试" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("命中阶段：最终出口"));
  expect(screen.getByRole("status")).toHaveTextContent("域名集外不按解析 IP 判断");
  expect(screen.getByRole("status")).toHaveTextContent("代理：Primary");
});

it("rejects invalid input and clears stale results after a backend failure", async () => {
  invoke.mockResolvedValue({
    stage: "china_ip",
    action: "direct",
    reason: "字面 IP 命中中国地址集",
    data_date: null,
  });
  render(<RouteTest />);
  const target = screen.getByRole("textbox", { name: "目标域名或 IP" });
  fireEvent.click(screen.getByRole("button", { name: "测试" }));
  expect(screen.getByRole("alert")).toHaveTextContent("请输入有效");
  fireEvent.change(target, { target: { value: "1.0.1.1" } });
  fireEvent.click(screen.getByRole("button", { name: "测试" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("直连"));
  invoke.mockRejectedValueOnce({ code: "unavailable", message: "规则集损坏", fields: [] });
  fireEvent.click(screen.getByRole("button", { name: "测试" }));
  await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("规则集损坏"));
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
});
