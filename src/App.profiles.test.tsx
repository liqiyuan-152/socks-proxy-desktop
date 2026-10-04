import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { fixture, mocks } from "./test/app-fixture";
import App from "./App";

describe("backend-driven desktop UI", () => {
  it("searches real profiles and preserves credentials when editing non-secret fields", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    await screen.findByRole("button", { name: "编辑Primary" });
    fireEvent.change(screen.getByPlaceholderText("搜索代理名称、服务器地址..."), {
      target: { value: "absent" },
    });
    expect(screen.getByText("暂无匹配的代理档案")).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText("搜索代理名称、服务器地址..."), {
      target: { value: "Primary" },
    });
    fireEvent.click(screen.getByRole("button", { name: "编辑Primary" }));
    await waitFor(() => expect(screen.getByLabelText("用户名")).toHaveValue("alice"));
    expect(screen.getByLabelText("密码")).toHaveValue("stored-secret");
    expect(screen.getByLabelText("密码")).toHaveAttribute("type", "password");
    fireEvent.change(screen.getByLabelText("服务器*"), { target: { value: "new.example.org" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "save_profile",
        expect.objectContaining({
          input: expect.objectContaining({
            host: "new.example.org",
            credential: { action: "preserve" },
          }),
        }),
      ),
    );
  });

  it("replaces edited credentials and clears them when the dialog closes", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    fireEvent.change(screen.getByLabelText("用户名"), { target: { value: "bob" } });
    fireEvent.change(screen.getByLabelText("密码"), { target: { value: "changed-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith(
        "save_profile",
        expect.objectContaining({
          input: expect.objectContaining({
            credential: { action: "replace", username: "bob", password: "changed-secret" },
          }),
        }),
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.click(screen.getByRole("switch", { name: "启用认证" }));
    expect(screen.getByLabelText("用户名")).toHaveValue("");
    expect(screen.getByLabelText("密码")).toHaveValue("");
  });

  it("does not fetch credentials for unauthenticated profiles", async () => {
    fixture.profiles[0].authentication_enabled = false;
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("get_profile_credential", expect.anything());
  });

  it("blocks saving when credential loading fails and retries successfully", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let attempt = 0;
    mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) => {
      if (name === "get_profile_credential" && attempt++ === 0) {
        return Promise.reject({ message: "凭据库不可用" });
      }
      return original(name, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("凭据库不可用");
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue("stored-secret"));
    expect(screen.getByRole("button", { name: "保存" })).not.toBeDisabled();
  });

  it("ignores credentials returned after closing the edit dialog", async () => {
    const original = mocks.invoke.getMockImplementation()!;
    let finish: ((credential: { username: string; password: string }) => void) | undefined;
    mocks.invoke.mockImplementation((name: string, args: Record<string, unknown>) =>
      name === "get_profile_credential"
        ? new Promise((resolve) => {
            finish = resolve;
          })
        : original(name, args),
    );
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "编辑Primary" }));
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));
    fireEvent.click(screen.getByRole("switch", { name: "启用认证" }));
    finish?.({ username: "alice", password: "stored-secret" });
    await waitFor(() => expect(screen.getByLabelText("密码")).toHaveValue(""));
  });

  it("distinguishes selected, enabled and disabled profile statuses on all screen sizes", async () => {
    fixture.profiles.push(
      { ...fixture.profiles[0], id: "secondary", name: "Secondary" },
      { ...fixture.profiles[0], id: "disabled", name: "Disabled", enabled: false },
    );
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    await screen.findByText("Secondary");

    for (const [name, label, variant] of [
      ["Primary", "默认代理", "success"],
      ["Secondary", "已启用", "success"],
      ["Disabled", "已停用", "secondary"],
    ]) {
      const row = within(screen.getByRole("table")).getByText(name).closest("tr")!;
      const labels = within(row).getAllByText(label);
      expect(labels).toHaveLength(2);
      for (const item of labels) expect(item).toHaveAttribute("data-variant", variant);
    }
  });

  it("puts the select action first for an inactive proxy", async () => {
    fixture.profiles.push({ ...fixture.profiles[0], id: "secondary", name: "Secondary" });
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    const select = await screen.findByRole("button", { name: "设Secondary为默认代理" });
    const actions = select.parentElement;
    expect(actions).toHaveClass("grid-cols-2");
    expect(
      Array.from(actions?.querySelectorAll("button") ?? [], (button) =>
        button.getAttribute("aria-label"),
      ),
    ).toEqual(["设Secondary为默认代理", "测试Secondary延迟", "编辑Secondary", "删除Secondary"]);
  });

  it("sets and clears the default proxy without disabling other exits", async () => {
    fixture.profiles.push({ ...fixture.profiles[0], id: "secondary", name: "Secondary" });
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    fireEvent.click(await screen.findByRole("button", { name: "设Secondary为默认代理" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "设Primary为默认代理" })).toBeInTheDocument(),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("select_profile", { id: "secondary" });
    expect(fixture.profiles.every((profile) => profile.enabled)).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "取消默认代理" }));
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "取消默认代理" })).not.toBeInTheDocument(),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("select_profile", { id: null });
  });

  it("keeps a referenced proxy enabled when disabling is rejected", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("link", { name: "代理" }));
    const switches = await screen.findAllByRole("switch", { name: "Primary启用状态" });
    fireEvent.click(switches[0]);
    expect(await screen.findByRole("alert")).toHaveTextContent("代理仍被默认出口或规则引用");
    expect(fixture.profiles[0].enabled).toBe(true);
    expect(switches[0]).toBeChecked();
  });
});
