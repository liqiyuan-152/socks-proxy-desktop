import { Component, type ReactNode } from "react";
import { ErrorAlert } from "@/components/ErrorAlert";

interface Props {
  children: ReactNode;
}
interface State {
  failed: boolean;
}

/** Recover a failed React tree without stopping the native proxy or reloading the WebView. */
export class AppErrorBoundary extends Component<Props, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="mx-auto max-w-xl p-6">
          <ErrorAlert
            title="界面出现异常"
            error={{
              code: "ui_error",
              message: "页面暂时无法显示，请尝试重新打开界面。",
              fields: [],
            }}
            onRetry={() => this.setState({ failed: false })}
          />
        </main>
      );
    }
    return this.props.children;
  }
}
