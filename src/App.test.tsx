import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the application entry point", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Socks Proxy" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Get started" })).toBeInTheDocument();
  });
});
