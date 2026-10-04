import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { fixture, mocks } from "./test/app-fixture";
import App from "./App";
import type { LatencyTask } from "./pages/proxies/latencyTask";

describe("backend-driven desktop UI", () => {
  it("tests one profile on demand and clears its result after editing", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    const test = await screen.findByRole("button", { name: "测试Primary延迟" });
    expect(test.closest("td")?.cellIndex).toBe(7);
    expect(screen.queryByText("42 ms")).not.toBeInTheDocument();
    fireEvent.click(test);
    await screen.findByText("42 ms");
    expect(await screen.findByText("Primary：42 ms")).toBeInTheDocument();
    expect(mocks.invoke).toHaveBeenCalledWith("start_proxy_latency_task", { id: "primary" });
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    fireEvent.change(screen.getByLabelText("服务器*"), { target: { value: "other.example.org" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(screen.queryByText("42 ms")).not.toBeInTheDocument());
  });

  it("shows test failures without fabricating a latency", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "start_proxy_latency_task"
        ? Promise.reject({ code: "unavailable", message: "代理测试超时（5 秒）", fields: [] })
        : original(command, args),
    );
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "测试Primary延迟" }));
    await screen.findByText("失败");
    expect(screen.getByText("失败")).toHaveAttribute(
      "title",
      expect.stringContaining("代理测试超时（5 秒）"),
    );
    expect(
      await screen.findByText("Primary 延迟测试失败：代理测试超时（5 秒）"),
    ).toBeInTheDocument();
  });

  it("shows test progress in a toast and disables retesting until completion", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((result: { latency_ms: number }) => void) | undefined;
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) =>
      command === "start_proxy_latency_task"
        ? new Promise<{ subscription_id: string; task: LatencyTask }>((resolve) => {
            finish = (result) =>
              resolve({
                subscription_id: "subscription-primary",
                task: {
                  task_id: "task-primary",
                  profile_id: "primary",
                  configuration_revision: fixture.profiles[0].configuration_revision,
                  state: "succeeded",
                  result,
                  error: null,
                },
              });
          })
        : original(command, args),
    );
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    const test = await screen.findByRole("button", { name: "测试Primary延迟" });
    fireEvent.click(test);
    expect(await screen.findByText("正在测试 Primary 的延迟…")).toBeInTheDocument();
    expect(test).toBeDisabled();

    finish?.({ latency_ms: 250 });
    expect(await screen.findByText("Primary：250 ms")).toBeInTheDocument();
    await waitFor(() => expect(test).not.toBeDisabled());
    expect(screen.getByText("250 ms")).toHaveClass("text-amber-700");
  });

  it("limits batch testing to three concurrent profiles", async () => {
    fixture.profiles = Array.from({ length: 5 }, (_, index) => ({
      ...fixture.profiles[0],
      id: `proxy-${index}`,
      name: `Proxy${index}`,
    }));
    const original = mocks.invoke.getMockImplementation()!;
    let active = 0;
    let maximum = 0;
    const pending: Array<() => void> = [];
    mocks.invoke.mockImplementation((command: string, args: Record<string, unknown>) => {
      if (command !== "start_proxy_latency_task") return original(command, args);
      active++;
      maximum = Math.max(maximum, active);
      return new Promise((resolve) =>
        pending.push(() => {
          active--;
          resolve({
            subscription_id: `subscription-${args.id}`,
            task: {
              task_id: `task-${args.id}`,
              profile_id: args.id,
              configuration_revision: fixture.profiles[0].configuration_revision,
              state: "succeeded",
              result: { latency_ms: 30 },
              error: null,
            },
          });
        }),
      );
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    const batch = await screen.findByRole("button", { name: "测试全部" });
    await waitFor(() => expect(batch).not.toBeDisabled());
    fireEvent.click(batch);
    await waitFor(() => expect(pending).toHaveLength(3));
    expect(maximum).toBe(3);
    pending[0]();
    await waitFor(() => expect(pending).toHaveLength(4));
    pending[1]();
    await waitFor(() => expect(pending).toHaveLength(5));
    pending.slice(2).forEach((finish) => finish());
    await waitFor(() => expect(screen.getAllByText("30 ms")).toHaveLength(5));
    expect(maximum).toBe(3);
  });

  it("fills a new profile from a link and saves only after confirmation", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "添加代理" })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.change(await screen.findByLabelText("代理链接"), {
      target: { value: "【socks5://repeatlink:abc0123456789\\@210.48.231.68:1080】" },
    });
    fireEvent.click(screen.getByRole("button", { name: "解析" }));
    expect(screen.getByLabelText("代理链接")).toHaveValue("");
    expect(screen.getByLabelText("名称*")).toHaveValue("210.48.231.68:1080");
    expect(screen.getByLabelText("服务器*")).toHaveValue("210.48.231.68");
    expect(screen.getByLabelText("端口*")).toHaveValue("1080");
    expect(screen.getByRole("switch", { name: "启用认证" })).toBeChecked();
    expect(screen.getByLabelText("用户名")).toHaveValue("repeatlink");
    expect(screen.getByLabelText("密码")).toHaveValue("abc0123456789");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "save_profile")).toHaveLength(
      0,
    );

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("save_profile", {
        input: expect.objectContaining({
          id: null,
          name: "210.48.231.68:1080",
          protocol: "socks5",
          host: "210.48.231.68",
          port: 1080,
          authentication_enabled: true,
          credential: { action: "replace", username: "repeatlink", password: "abc0123456789" },
        }),
      }),
    );
  });

  it("keeps manual fields on parse failure and hides the link field when editing", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "添加代理" })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    await screen.findByLabelText("代理链接");
    fireEvent.change(screen.getByLabelText("名称*"), { target: { value: "Manual" } });
    fireEvent.change(screen.getByLabelText("代理链接"), {
      target: { value: "socks5://host:65536" },
    });
    fireEvent.click(screen.getByRole("button", { name: "解析" }));
    expect(screen.getByRole("alert")).toHaveTextContent("端口");
    expect(screen.getByLabelText("名称*")).toHaveValue("Manual");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "save_profile")).toHaveLength(
      0,
    );
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    expect(screen.queryByLabelText("代理链接")).not.toBeInTheDocument();
  });
});
