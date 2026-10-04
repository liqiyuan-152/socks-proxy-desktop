import { render, screen } from "@testing-library/react";
import { DailyConnectionTrend } from "./DailyConnectionTrend";

const yesterday = { yesterday_date: "2026-10-03", yesterday_count_sum: 8, yesterday_samples: 2 };

it("compares the current count with the persisted yesterday mean", () => {
  const view = render(<DailyConnectionTrend count={6} trend={yesterday} />);
  expect(screen.getByText("+50.0%")).toBeInTheDocument();
  expect(screen.getByText(/均值 4.0 · 2 次采样/)).toBeInTheDocument();
  view.rerender(<DailyConnectionTrend count={2} trend={yesterday} />);
  expect(screen.getByText("-50.0%")).toBeInTheDocument();
  view.rerender(<DailyConnectionTrend count={4} trend={yesterday} />);
  expect(screen.getByText("0.0%")).toBeInTheDocument();
});

it("distinguishes unavailable history from a real zero baseline", () => {
  const view = render(<DailyConnectionTrend count={5} trend={null} />);
  expect(screen.getByText("暂无昨日采样")).toBeInTheDocument();
  view.rerender(<DailyConnectionTrend count={null} trend={yesterday} />);
  expect(screen.getByText("当前连接观测不可用")).toBeInTheDocument();
  view.rerender(
    <DailyConnectionTrend count={5} trend={{ ...yesterday, yesterday_count_sum: 0 }} />,
  );
  expect(screen.getByText("增加 5 个（昨日为零）")).toBeInTheDocument();
  view.rerender(
    <DailyConnectionTrend count={0} trend={{ ...yesterday, yesterday_count_sum: 0 }} />,
  );
  expect(screen.getByText("持平")).toBeInTheDocument();
});
