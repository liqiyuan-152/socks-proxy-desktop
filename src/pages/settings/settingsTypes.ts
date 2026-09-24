export type Retention = "days7" | "days30" | "days90" | "permanent";

export type Settings = {
  launch_at_login: boolean;
  diagnostic_retention: Retention;
  latency_test_url: string;
};
