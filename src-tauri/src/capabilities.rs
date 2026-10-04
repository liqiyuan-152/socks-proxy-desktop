use serde::Serialize;

#[derive(Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Capabilities {
    pub platform: &'static str,
    pub proxy_runtime: bool,
    pub proxy_latency: bool,
    pub network_recovery: bool,
    pub startup: bool,
}

#[tauri::command]
pub fn get_capabilities() -> Capabilities {
    Capabilities {
        platform: std::env::consts::OS,
        proxy_runtime: cfg!(windows),
        proxy_latency: cfg!(windows),
        network_recovery: cfg!(windows),
        startup: cfg!(windows),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn capabilities_describe_the_compiled_platform() {
        let value = super::get_capabilities();
        assert_eq!(value.platform, std::env::consts::OS);
        assert_eq!(value.proxy_runtime, cfg!(windows));
        assert_eq!(value.proxy_latency, cfg!(windows));
        assert_eq!(value.network_recovery, cfg!(windows));
        assert_eq!(value.startup, cfg!(windows));
    }
}
