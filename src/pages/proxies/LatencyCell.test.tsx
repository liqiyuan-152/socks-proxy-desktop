import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { LatencyCell } from "./LatencyCell";

it("colors measured latency at the green, yellow and red boundaries", () => {
  const { rerender } = render(<LatencyCell result={{ latency: 200, at: 1000 }} />);
  expect(screen.getByText("200 ms")).toHaveClass("text-emerald-700");

  rerender(<LatencyCell result={{ latency: 201, at: 1000 }} />);
  expect(screen.getByText("201 ms")).toHaveClass("text-amber-700");

  rerender(<LatencyCell result={{ latency: 500, at: 1000 }} />);
  expect(screen.getByText("500 ms")).toHaveClass("text-amber-700");

  rerender(<LatencyCell result={{ latency: 501, at: 1000 }} />);
  expect(screen.getByText("501 ms")).toHaveClass("text-red-700");
});

it("shows pending and failed tests without assigning a latency range", () => {
  const { rerender } = render(<LatencyCell result={{ pending: true }} />);
  expect(screen.getByText("测试中…")).toHaveClass("text-muted-foreground");

  rerender(<LatencyCell result={{ error: "测试超时", at: 1000 }} />);
  expect(screen.getByText("失败")).toHaveClass("text-destructive");
  expect(screen.getByText("失败")).toHaveAttribute("title", expect.stringContaining("测试超时"));
});
