//! 保持 IPC 兼容的国内直连状态视图。
use serde::Serialize;
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
/// 国内直连开关、规则集可用性和数据日期。
pub struct ChinaDirectStatus {
    pub enabled: bool,
    pub available: bool,
    pub data_date: Option<String>,
}
