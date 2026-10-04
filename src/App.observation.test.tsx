import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fixture, mocks } from "./test/app-fixture";
import App from "./App";

describe("backend-driven desktop UI", () => {
  it("loads real rules and persists enable toggles and ordering", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "分流规则" }));
    await screen.findByText("共 1 条规则");
    fireEvent.click(screen.getByRole("switch", { name: "Example已启用" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("replace_rules", {
        rules: [expect.objectContaining({ id: "rule-1", enabled: false })],
      }),
    );
  });

  it("saves a rule with its selected exit and port range", async () => {
    fixture.profiles.push({ ...fixture.profiles[0], id: "secondary", name: "Secondary" });
    fixture.rules[0].proxy_profile_id = "secondary";
    fixture.rules[0].port_end = 445;
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "分流规则" }));
    await screen.findByText("443–445");
    expect(screen.getByText("Secondary")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "编辑Example" }));
    expect(screen.getByRole("textbox", { name: "结束端口" })).toHaveValue("445");
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("replace_rules", {
        rules: [
          expect.objectContaining({
            id: "rule-1",
            proxy_profile_id: "secondary",
            port_start: 443,
            port_end: 445,
          }),
        ],
      }),
    );
  });

  it("shows unavailable history, active details, and confirms diagnostic cleanup", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "连接日志" }));
    await screen.findByText("代理模式已提交");
    expect(screen.getByText(/已完成连接历史、成功率与失败详情暂不可用/)).toBeInTheDocument();
    expect(mocks.invoke).toHaveBeenCalledWith(
      "get_runtime_diagnostics",
      expect.objectContaining({
        filter: expect.objectContaining({
          severity: null,
          until_ms: null,
          from_ms: expect.any(Number),
        }),
        offset: 0,
        limit: 100,
      }),
    );
    fireEvent.change(screen.getByPlaceholderText("搜索目标、规则或诊断..."), {
      target: { value: "absent" },
    });
    expect(screen.getByText("暂无运行时诊断")).toBeInTheDocument();
    expect(screen.getByText("暂无活跃连接")).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText("搜索目标、规则或诊断..."), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "清理诊断" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("活跃连接不受影响");
    fireEvent.click(screen.getByRole("button", { name: "确认清理" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "clear_runtime_diagnostics",
        expect.objectContaining({ confirmed: true }),
      ),
    );
    expect(screen.getByText("example.org:443")).toBeInTheDocument();
  });

  it("opens more details and copies only the backend-provided active connection summary", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<App />);
    await screen.findByText("example.org");
    fireEvent.click(screen.getByRole("button", { name: /查看更多/ }));
    await screen.findByText("代理模式已提交");
    expect(screen.getByText(/已完成连接历史、成功率与失败详情暂不可用/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看example.org详情" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("最终结果不可用");
    fireEvent.click(screen.getByRole("button", { name: "复制脱敏详情" }));
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith("目标: example.org:443\n出口链: selected-proxy"),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("copy_active_connection_detail", { id: "active-1" });
  });

  it("renders empty backend snapshots without invented connections or results", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command === "get_active_connections")
        return Promise.resolve({
          status: "available",
          active_count: 0,
          history_available: false,
          trend: null,
          diagnostic: null,
          recent: [],
        });
      return original(command, args);
    });
    render(<App />);
    await screen.findByText("暂无活跃连接");
    expect(screen.getByRole("heading", { name: "当前活跃连接" })).toBeInTheDocument();
    expect(screen.queryByText("example.org")).not.toBeInTheDocument();
  });
});
