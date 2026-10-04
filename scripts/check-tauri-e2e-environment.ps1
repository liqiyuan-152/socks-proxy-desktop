param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = 'stable'
Set-Location $repository
$toolsRoot = Join-Path $repository '.e2e-tools'
$binary = Join-Path $repository 'src-tauri\target\debug\socks-proxy.exe'
$config = Join-Path $repository 'src-tauri\tauri.e2e.conf.json'
$identifier = 'com.socksproxy.desktop.architecture-e2e'
$configuration = Get-Content $config -Raw | ConvertFrom-Json
if ($configuration.identifier -cne $identifier) { throw 'E2E 配置必须使用固定隔离应用标识' }
$configHash = (Get-FileHash $config -Algorithm SHA256).Hash.ToLowerInvariant()
if ($SkipBuild) {
    $previousPath = Join-Path $toolsRoot 'preflight.json'
    if (-not (Test-Path $previousPath) -or -not (Test-Path $binary)) { throw '缺少隔离构建预检记录，不能跳过构建' }
    $previous = Get-Content $previousPath -Raw | ConvertFrom-Json
    $binaryHash = (Get-FileHash $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($previous.status -cne 'passed' -or $previous.application_identifier -cne $identifier -or
        $previous.application_sha256 -cne $binaryHash -or $previous.configuration_sha256 -cne $configHash) {
        throw '二进制或配置与已验证的隔离构建不匹配，不能跳过构建'
    }
}
& (Join-Path $PSScriptRoot 'setup-tauri-e2e.ps1')
if (-not $SkipBuild) {
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        pnpm tauri build --no-bundle --debug --config src-tauri/tauri.e2e.conf.json
        $result = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($result -ne 0) { throw '隔离 E2E 应用构建失败' }
}
$environment = Get-Content (Join-Path $toolsRoot 'environment.json') -Raw | ConvertFrom-Json
if (-not (Test-Path $binary)) { throw '隔离应用二进制不存在' }
$process = Start-Process -FilePath $environment.tauri_driver -ArgumentList @(
    '--port', '4445', '--native-port', '4446', '--native-driver', ('"' + $environment.native_driver + '"')
) -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $toolsRoot 'driver-stdout.log') -RedirectStandardError (Join-Path $toolsRoot 'driver-stderr.log')
$null = $process.Handle
try {
    $ready = $null
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($process.HasExited) { throw 'Tauri WebDriver 提前退出' }
        try {
            # 只读取服务状态，不 POST /session，不启动/操作应用界面。
            $ready = Invoke-RestMethod -Uri 'http://127.0.0.1:4445/status' -TimeoutSec 2
            if ($ready.value.ready -eq $true) { break }
        } catch { }
        Start-Sleep -Milliseconds 250
    }
    if ($ready.value.ready -ne $true) { throw 'Tauri/Edge WebDriver 未就绪' }
    $listener = Get-NetTCPConnection -LocalPort 4445 -State Listen -ErrorAction Stop
    if ($listener.OwningProcess -ne $process.Id) { throw '服务端口不属于本次启动的驱动进程' }
    [ordered]@{
        status = 'passed'
        application = $binary
        application_sha256 = (Get-FileHash $binary -Algorithm SHA256).Hash.ToLowerInvariant()
        application_identifier = $identifier
        configuration_sha256 = $configHash
        webdriver = $environment
        server_response = $ready
        ui_session_created = $false
        scope = '官方驱动与原生隔离应用构建；只验证服务就绪，不等同用户 E2E 场景'
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $toolsRoot 'preflight.json') -Encoding utf8
    Write-Output 'E2E_ENVIRONMENT_READY ui_session_created=false'
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -ErrorAction SilentlyContinue }
    $process.Dispose()
}
