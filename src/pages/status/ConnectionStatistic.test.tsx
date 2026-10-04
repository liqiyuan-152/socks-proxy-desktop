import { render, screen } from "@testing-library/react";
import { ActiveConnectionTrend } from "./ConnectionStatistic";

it("compares actual count changes and handles missing or zero baselines", () => {
  const view = render(<ActiveConnectionTrend count={null} />);
  expect(screen.getByText("暂无可比较的采样")).toBeInTheDocument();
  view.rerender(<ActiveConnectionTrend count={0} />);
  expect(screen.getByText("等待下一次连接数变化")).toBeInTheDocument();
  view.rerender(<ActiveConnectionTrend count={2} />);
  expect(screen.getByText("新增 2 个")).toBeInTheDocument();
  view.rerender(<ActiveConnectionTrend count={1} />);
  expect(screen.getByText("-50.0%")).toBeInTheDocument();
  view.rerender(<ActiveConnectionTrend count={null} />);
  view.rerender(<ActiveConnectionTrend count={10} />);
  expect(screen.getByText("等待下一次连接数变化")).toBeInTheDocument();
});
