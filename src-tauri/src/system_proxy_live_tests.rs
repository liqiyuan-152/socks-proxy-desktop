//! 显式运行的真实 Windows 用户代理恢复测试；不纳入普通回归。
use super::WindowsProxyDevice;
use crate::{
    runtime_session::{SessionLease, RUNTIME_SESSION_KEY},
    store::SqliteConfigurationStore,
    system_proxy::{
        OwnedSystemProxy, ProxyOwnershipStore, ProxySettingsDevice, SystemProxyAdapter,
    },
};
use std::{error::Error, sync::Arc};

struct RestoreOnExit<'a>(&'a OwnedSystemProxy);
impl Drop for RestoreOnExit<'_> {
    fn drop(&mut self) {
        // 只恢复仍能确认归属的设置；不能覆盖测试期间其他程序的改动。
        let _ = self.0.restore_if_owned();
    }
}

#[test]
#[ignore = "需要明确执行：短暂接管当前 Windows 用户代理，并验证完整恢复"]
fn live_user_proxy_restores_original_settings_after_stop_and_reopen() -> Result<(), Box<dyn Error>>
{
    if std::env::var("SOCKS_PROXY_LIVE_PROXY_VALIDATION").as_deref() != Ok("1") {
        return Err("真实系统代理验证需要显式环境开关".into());
    }
    let _lease = SessionLease::acquire(RUNTIME_SESSION_KEY)?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("proxy-recovery.sqlite3");
    let store = Arc::new(SqliteConfigurationStore::open(&path)?);
    let device = WindowsProxyDevice::default();
    let original = device.read()?;
    // 监听端口保持存在，测试不依赖任何外部网站或真实上游代理。
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let adapter = OwnedSystemProxy::new(
        Box::new(WindowsProxyDevice::default()),
        Box::new(store.clone()),
    );
    let cleanup = RestoreOnExit(&adapter);
    adapter.enable(port)?;
    let applied = device.read()?;
    assert!(applied.enabled == Some(1), "未接管用户系统代理");
    assert!(
        applied.server.as_deref() == Some(format!("127.0.0.1:{port}").as_str()),
        "监听端口未应用"
    );
    adapter.restore_if_owned()?;
    assert!(device.read()? == original, "停止后没有完整恢复原设置");
    assert!(store.load_ownership()?.is_none(), "恢复记录未清理");

    adapter.enable(port)?;
    // 重新创建 adapter/database 验证持久化恢复记录，退出保护始终保留。
    let reopened = OwnedSystemProxy::new(
        Box::new(WindowsProxyDevice::default()),
        Box::new(SqliteConfigurationStore::open(&path)?),
    );
    reopened.recover_on_startup()?;
    assert!(device.read()? == original, "重建适配器后没有完整恢复原设置");
    assert!(store.load_ownership()?.is_none(), "重建恢复后记录未清理");
    drop(cleanup);
    Ok(())
}
