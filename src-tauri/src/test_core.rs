/// Optional for unit-only runs; mandatory in the verified real-core gate.
pub(crate) fn binary() -> Option<String> {
    match std::env::var("SING_BOX_TEST_BIN") {
        Ok(binary) if !binary.is_empty() => {
            assert!(std::path::Path::new(&binary).is_file(), "测试内核不存在");
            Some(binary)
        }
        _ => {
            assert!(
                std::env::var_os("REQUIRE_SING_BOX_TEST_BIN").is_none(),
                "真实内核测试要求 SING_BOX_TEST_BIN"
            );
            None
        }
    }
}
