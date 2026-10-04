# 前端状态管理

`BackendProvider` 创建独立 Zustand store 并管理初始化、运行时事件订阅、轮询及清理。组件通过 `useBackendStore(selector)` 订阅所需字段，多个字段使用 `useShallow`；`selectSelectedMode`、`selectIsRunning`、`selectAppliedMode` 提供派生状态。旧 BackendContext 和观察、模式切换、轮询 hook 已移除。

读取按资源递增请求版本；响应只有在同一生命周期中且仍是最新请求时才更新数据。运行时事件使旧快照读取失效。写操作期间暂停快照轮询应用，写入成功后刷新共享数据。页面卸载不取消已经提交给后端的写入，其结果仍刷新应用级 store；页面自身的表单反馈通过 mounted 标志阻止卸载后更新。

模式切换使用串行 drain，同一轮的调用者等待同一 Promise，连续选择只保留最后一个尚未执行的模式。失败保留用户选择和完整 AppError；恢复读取失败时保留最近一次权威快照。新成功操作或主动重试清除旧失败。

窗口隐藏时暂停轮询，重新可见或获得焦点时立即读取，慢请求不会重叠。Provider 清理使所有旧请求失效，取消未执行的模式选择，并释放计时器与事件监听。StrictMode 的重复 setup/cleanup 不会留下旧订阅。

store 持有代理视图、规则、运行时快照、能力和连接信息；凭据仅作为 IPC 调用参数，不保存到 store。表单草稿及读取到的认证信息仍由表单持有，关闭时清空。开发模式启用 Zustand devtools，生产构建禁用；devtools 中只出现不含凭据的共享状态。
