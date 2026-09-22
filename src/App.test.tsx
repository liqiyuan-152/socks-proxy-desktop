import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach } from "vitest";
import App from "./App";

describe("App", () => {
  beforeEach(() => {
    window.history.pushState({}, "", "/");
  });

  it("renders the Socks Proxy status dashboard", () => {
    render(<App />);

    expect(screen.getByRole("link", { name: "状态" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "当前代理信息" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "最近连接记录" })).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getByText("api.remote.net")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "规则代理" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "全局代理" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "全局直连" })).toBeInTheDocument();
  });

  it("switches to the static proxy management view", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "代理" }));

    expect(screen.getByRole("link", { name: "代理" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "代理" })).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: /公司代理/ })).toBeInTheDocument();
    expect(screen.getByText("连接失败")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "编辑公司代理" })).toBeInTheDocument();
  });

  it("opens and closes the add proxy dialog without changing the static list", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(screen.getByRole("button", { name: "添加代理" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "添加代理" })).toBeInTheDocument();
    expect(screen.getByLabelText("名称*")).toHaveValue("");
    expect(screen.getByPlaceholderText("请输入代理名称")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("例如：proxy.example.com")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("例如：1080")).toBeInTheDocument();
    expect(screen.queryByLabelText("用户名")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("cell", { name: /公司代理/ })).toBeInTheDocument();
  });

  it("prefills the edit dialog and controls authentication credentials", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑公司代理" }));

    expect(screen.getByRole("heading", { name: "编辑代理" })).toBeInTheDocument();
    expect(screen.getByLabelText("名称*")).toHaveValue("公司代理");
    expect(screen.getByLabelText("服务器*")).toHaveValue("proxy.example.com");
    expect(screen.getByLabelText("用户名")).toHaveValue("user123");

    const password = screen.getByLabelText("密码");
    expect(password).toHaveAttribute("type", "password");
    fireEvent.click(screen.getByRole("button", { name: "显示密码" }));
    expect(password).toHaveAttribute("type", "text");

    fireEvent.click(screen.getByRole("switch", { name: "启用认证" }));
    expect(screen.queryByLabelText("用户名")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("密码")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("switches to the static routing rules view", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "分流规则" }));

    expect(screen.getByRole("link", { name: "分流规则" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "分流规则" })).toBeInTheDocument();
    expect(screen.getByPlaceholderText("搜索规则名称或目标...")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "添加规则" })).toBeInTheDocument();
    expect(screen.getByText("共 4 条规则")).toBeInTheDocument();
    expect(screen.getByText("173.30.0.0/16")).toBeInTheDocument();
    expect(screen.getByText("example.com")).toBeInTheDocument();
    expect(screen.getAllByRole("switch", { checked: true })).toHaveLength(4);
  });

  it("switches to the static connection logs view", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "连接日志" }));

    expect(screen.getByRole("link", { name: "连接日志" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "连接日志" })).toBeInTheDocument();
    expect(screen.getByPlaceholderText("搜索目标地址、规则、错误信息...")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "目标" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "结果" })).toBeInTheDocument();
    expect(screen.getAllByText("bad.example.com")).toHaveLength(2);
    expect(screen.getByText("连接超时（Connection timed out）")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "复制日志详情" })).toBeInTheDocument();
  });

  it("renders a content view from its URL", () => {
    window.history.pushState({}, "", "/logs");

    render(<App />);

    expect(screen.getByRole("link", { name: "连接日志" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "连接日志" })).toBeInTheDocument();
  });

  it("opens and closes the static add rule dialog", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "分流规则" }));
    fireEvent.click(screen.getByRole("button", { name: "添加规则" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "添加规则" })).toBeInTheDocument();
    expect(screen.getByLabelText("规则名称")).toHaveValue("常用站点");
    expect(screen.getByLabelText("目标值")).toHaveValue("example.com");
    expect(screen.getByLabelText("端口")).toHaveValue("443");

    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByText("共 4 条规则")).toBeInTheDocument();
  });

  it("prefills the edit rule dialog and supports cancelling it", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "分流规则" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑常用站点" }));

    expect(screen.getByRole("heading", { name: "编辑规则" })).toBeInTheDocument();
    expect(screen.getByLabelText("规则名称")).toHaveValue("常用站点");
    expect(screen.getByLabelText("目标值")).toHaveValue("example.com");
    expect(screen.getByLabelText("端口")).toHaveValue("443");
    expect(screen.getByRole("button", { name: "代理" })).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(screen.getByRole("button", { name: "取消" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("updates the active mode when a proxy mode is selected", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("tab", { name: "全局代理" }));

    expect(screen.getByTestId("active-mode")).toHaveTextContent("全局代理");
    expect(screen.getByText("全部流量经代理")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("link", { name: "代理" }));
    fireEvent.click(screen.getByRole("link", { name: "状态" }));

    expect(screen.getByTestId("active-mode")).toHaveTextContent("全局代理");
  });

  it("renders the settings page and updates its session-only controls", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "设置" }));

    expect(screen.getByRole("link", { name: "设置" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "设置" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "启动" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "配置备份" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "日志" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "网络恢复" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "关于" })).toBeInTheDocument();
    expect(screen.getByText("v1.2.0")).toBeInTheDocument();
    expect(screen.getByText("MIT License")).toBeInTheDocument();

    const launchAtLogin = screen.getByRole("switch", { name: "开机启动" });
    expect(launchAtLogin).toHaveAttribute("data-state", "checked");
    fireEvent.click(launchAtLogin);
    expect(launchAtLogin).toHaveAttribute("data-state", "unchecked");

    fireEvent.change(screen.getByLabelText("日志保留策略"), { target: { value: "90" } });
    expect(screen.getByLabelText("日志保留策略")).toHaveValue("90");
  });

  it("confirms destructive settings actions and resets default settings", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    fireEvent.click(screen.getByRole("button", { name: "清空选择日志" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "清空选择日志" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("switch", { name: "开机启动" }));
    fireEvent.change(screen.getByLabelText("日志保留策略"), { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: "恢复全部重置" }));
    fireEvent.click(screen.getByRole("button", { name: "恢复默认" }));

    expect(screen.getByRole("switch", { name: "开机启动" })).toHaveAttribute(
      "data-state",
      "checked",
    );
    expect(screen.getByLabelText("日志保留策略")).toHaveTextContent("保留 30 天");
  });

  it("shows non-mutating prototype feedback for backup and update controls", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("link", { name: "设置" }));
    fireEvent.click(screen.getByRole("button", { name: "导出配置" }));
    expect(screen.getByText("这是界面原型，当前不会创建或写入配置文件。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "知道了" }));

    fireEvent.click(screen.getByRole("button", { name: "检查更新" }));
    expect(screen.getByText("这是界面原型，当前不会发起网络请求检查新版本。")).toBeInTheDocument();
  });
});
