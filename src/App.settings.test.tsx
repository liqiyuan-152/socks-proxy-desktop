import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fixture, mocks } from "./test/app-fixture";
import App from "./App";

describe("backend-driven desktop UI", () => {
  it("persists settings and hides update checks while network recovery is unavailable", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "设置" }));
    const startup = await screen.findByRole("switch", { name: "开机启动" });
    await waitFor(() => expect(startup).not.toBeDisabled());
    fireEvent.click(startup);
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_settings", {
        settings: {
          launch_at_login: true,
          diagnostic_retention: "days30",
          latency_test_url: "https://www.gstatic.com/generate_204",
        },
      }),
    );
    expect(screen.queryByRole("button", { name: "检查更新" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "恢复网络设置" })).toBeDisabled();
    expect(screen.queryByText("尚未配置发布渠道，无法检查更新。")).not.toBeInTheDocument();
  });

  it("saves the configured latency test URL", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "设置" }));
    const input = await screen.findByRole("textbox", { name: "延迟测试地址" });
    fireEvent.change(input, { target: { value: "https://example.org/check" } });
    fireEvent.click(screen.getByRole("button", { name: "保存测试地址" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("update_settings", {
        settings: { ...fixture.settings, latency_test_url: "https://example.org/check" },
      }),
    );
  });

  it("requires confirmation before clearing settings diagnostics", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "设置" }));
    fireEvent.click(await screen.findByRole("button", { name: "清理运行时诊断" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("不会清理或伪造活跃连接");
    expect(mocks.invoke).not.toHaveBeenCalledWith("clear_runtime_diagnostics", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("clear_runtime_diagnostics", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "清理运行时诊断" }));
    fireEvent.click(screen.getByRole("button", { name: "确认清空" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("clear_runtime_diagnostics", {
        filter: { from_ms: null, until_ms: null, severity: null },
        confirmed: true,
      }),
    );
  });

  it("requires fresh credentials when confirming an imported authenticated profile", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "设置" }));
    await screen.findByRole("switch", { name: "开机启动" });
    const fileInput = screen.getByLabelText("选择配置文件");
    expect(fileInput).toHaveClass("sr-only");
    expect(fileInput).not.toHaveClass("w-full");
    const json = JSON.stringify({
      schema_version: 1,
      profiles: [
        {
          id: "imported",
          name: "Imported",
          authentication_enabled: true,
        },
      ],
      rules: [],
    });
    const file = new File([json], "settings.json", { type: "application/json" });
    Object.defineProperty(file, "text", { value: async () => json });
    fireEvent.change(screen.getByLabelText("选择配置文件"), { target: { files: [file] } });
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "确认导入" }));
    expect(screen.getByRole("alert")).toHaveTextContent("重新输入认证用户名和密码");
    expect(mocks.invoke).not.toHaveBeenCalledWith("import_configuration", expect.anything());
    fireEvent.change(screen.getByRole("textbox", { name: "Imported用户名" }), {
      target: { value: "alice" },
    });
    fireEvent.change(screen.getByLabelText("Imported密码"), { target: { value: "fresh-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "确认导入" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("import_configuration", {
        json,
        updates: { imported: { action: "replace", username: "alice", password: "fresh-secret" } },
      }),
    );
    expect(screen.queryByText("fresh-secret")).not.toBeInTheDocument();
  });

  it("confirms Windows network recovery before invoking the backend", async () => {
    fixture.recoveryAvailable = true;
    {
      render(<App />);
      fireEvent.click(await screen.findByRole("link", { name: "设置" }));
      const restore = await screen.findByRole("button", { name: "恢复网络设置" });
      expect(restore).not.toBeDisabled();
      fireEvent.click(restore);
      expect(screen.getByRole("dialog")).toHaveTextContent("外部修改不会被覆盖");
      expect(mocks.invoke).not.toHaveBeenCalledWith("recover_network", expect.anything());
      fireEvent.click(screen.getByRole("button", { name: "确认恢复" }));
      await waitFor(() =>
        expect(mocks.invoke).toHaveBeenCalledWith("recover_network", { confirmed: true }),
      );
      await screen.findByText(/网络恢复检查完成/);
    }
  });
});

it("disables platform-dependent controls when the capability request fails", async () => {
  const original = mocks.invoke.getMockImplementation()!;
  mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
    if (name === "get_capabilities") return Promise.reject(new Error("unavailable"));
    return original(name, args);
  });
  render(<App />);
  await waitFor(() => expect(screen.getByRole("tab", { name: "全局代理" })).toBeDisabled());
  fireEvent.click(await screen.findByRole("link", { name: "代理" }));
  expect(await screen.findByRole("button", { name: "测试Primary延迟" })).toBeDisabled();
  fireEvent.click(await screen.findByRole("link", { name: "设置" }));
  expect(await screen.findByRole("switch", { name: "开机启动" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "恢复网络设置" })).toBeDisabled();
});
