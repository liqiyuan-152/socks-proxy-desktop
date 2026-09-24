# 国内直连规则集来源清单

此资源供可选的“国内直连”预设使用。数据代表域名列表和地址归属分类，不是 GFW 可达性检测，也不保证所有中国站点都被收录。运行时不远程更新；应用升级时需重新固定上游提交、校验输入并重新生成资源。

| 资源 | 来源与固定版本 | 许可 | 包内路径 |
| --- | --- | --- | --- |
| 域名 | `v2fly/domain-list-community` 的 `cn` 列表，提交 `c1c2cf0d252871e8739747714df06e8d2671f72f`（2026-09-22） | MIT | `china-rules/china-domains.srs` |
| IPv4/IPv6 | `gaoyifan/china-operator-ip` 的 `china46.txt`，`ip-lists` 提交 `8046f18143a4ad0ced43fe5d7e72e0c68ba73ac8`（2026-09-23） | MIT | `china-rules/china-ipv4.srs`、`china-rules/china-ipv6.srs` |

固定输入 URL、SHA-256、产物 SHA-256、条目数及许可证 SHA-256 记录在 [`manifest.json`](../src-tauri/resources/china-rules/manifest.json)。两个 MIT 许可证原文与 `.srs` 一起打包。IP 数据将可信国内 BGP 分类和未宣告的 RIR-CN 登记地址合并，归属可能滞后或存在误差；不把所有未收录地址断言为“国外”。

生成：`node scripts/prepare-china-rules.mjs`。网络不可用时，先提供固定版本的 `domains.zip`、`china46.txt`、`ip-LICENSE`，再执行 `node scripts/prepare-china-rules.mjs --source-dir <目录>`。两种模式都校验输入 SHA-256，使用锁定的 sing-box 1.14.1 编译；源列表中的 `include` 和属性筛选按上游语义展开，遇到不支持的语法时拒绝生成。

验证：设置 `SING_BOX_TEST_BIN` 为锁定内核路径，运行 `cargo test --manifest-path src-tauri/Cargo.toml --lib bundled_china_rules_check_and_load_offline_with_pinned_core` 及 `cargo test --manifest-path src-tauri/Cargo.toml --lib china_preset_`。测试核对资源和许可证哈希、域名及 IPv4/IPv6 命中、离线加载和受管入口连接。域名集外的域名原样交由默认 HTTP 代理，由上游自行解析；集成测试使用模拟混合 A/AAAA 候选、失败和地址族重试的上游，验证多次入口请求仍选择同一出口，不声称已验证最终拨号 IP 或真实公网 DNS 结果。
