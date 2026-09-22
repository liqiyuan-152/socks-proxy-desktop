import { fireEvent, render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the Socks Proxy status dashboard", () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "状态" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("heading", { name: "当前代理信息" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "最近连接记录" })).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getByText("api.remote.net")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "规则代理" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "全局代理" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "全局直连" })).toBeInTheDocument();
  });

  it("updates the active mode when a proxy mode is selected", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("tab", { name: "全局代理" }));

    expect(screen.getByTestId("active-mode")).toHaveTextContent("全局代理");
    expect(screen.getByText("全部流量经代理")).toBeInTheDocument();
  });
});
