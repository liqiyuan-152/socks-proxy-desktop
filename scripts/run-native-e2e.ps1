param(
    [switch]$RunUi, [switch]$ResetFixture, [switch]$SkipBuild,
    [ValidateSet('journey', 'runtime')][string]$Suite = 'journey',
    [ValidateRange(1, 86400)][int]$DurationSeconds = 1800
)

$ErrorActionPreference = 'Stop'
if (-not $RunUi) { throw '本脚本会执行真实 Windows UI；请明确传入 -RunUi。仅预检请使用 check-tauri-e2e-environment.ps1。' }
$repository = Split-Path $PSScriptRoot -Parent
Set-Location $repository
& (Join-Path $PSScriptRoot 'check-tauri-e2e-environment.ps1') -SkipBuild:$SkipBuild
$toolsRoot = Join-Path $repository '.e2e-tools'
$environment = Get-Content (Join-Path $toolsRoot 'environment.json') -Raw | ConvertFrom-Json
$reportDirectory = Join-Path $env:TEMP ('socks-native-e2e-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $reportDirectory | Out-Null

function Read-UserProxy {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Internet Settings')
    try {
        $values = [ordered]@{}
        foreach ($name in @('ProxyEnable', 'ProxyServer', 'ProxyOverride', 'AutoConfigURL', 'AutoDetect')) {
            $present = $key.GetValueNames() -contains $name
            $values[$name] = [ordered]@{
                present = $present
                kind = if ($present) { $key.GetValueKind($name).ToString() } else { $null }
                value = $key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            }
        }
        return ($values | ConvertTo-Json -Depth 5 -Compress)
    } finally { if ($key) { $key.Dispose() } }
}

$before = Read-UserProxy
$before | Set-Content (Join-Path $reportDirectory 'proxy-before.json') -Encoding utf8
$process = Start-Process -FilePath $environment.tauri_driver -ArgumentList @(
    '--port', '4445', '--native-port', '4446', '--native-driver', ('"' + $environment.native_driver + '"')
) -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $reportDirectory 'driver-stdout.log') -RedirectStandardError (Join-Path $reportDirectory 'driver-stderr.log')
$null = $process.Handle
try {
    $ready = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($process.HasExited) { throw 'Tauri WebDriver 提前退出' }
        try {
            $status = Invoke-RestMethod -Uri 'http://127.0.0.1:4445/status' -TimeoutSec 2
            if ($status.value.ready -eq $true) { $ready = $true; break }
        } catch { }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw '驱动未就绪' }
    $listener = Get-NetTCPConnection -LocalPort 4445 -State Listen -ErrorAction Stop
    if ($listener.OwningProcess -ne $process.Id) { throw '驱动端口不属于本次进程' }
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $arguments = @('scripts/e2e/run-windows.mjs', '--run-ui', '--report-directory', $reportDirectory,
            '--suite', $Suite, '--duration-seconds', [string]$DurationSeconds)
        if ($ResetFixture) { $arguments += '--reset-fixture' }
        node @arguments
        $result = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    $after = Read-UserProxy
    $after | Set-Content (Join-Path $reportDirectory 'proxy-after.json') -Encoding utf8
    $report = Get-Content (Join-Path $reportDirectory 'result.json') -Raw -Encoding utf8 | ConvertFrom-Json
    $restored = $before -ceq $after
    $report | Add-Member -NotePropertyName user_proxy_restored -NotePropertyValue $restored
    if (-not $restored) { $report.status = 'failed' }
    $report | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $reportDirectory 'result.json') -Encoding utf8
    if ($result -ne 0 -or -not $restored) { throw "E2E 失败或系统代理未恢复：$reportDirectory" }
    Write-Output "NATIVE_E2E_PASSED report=$reportDirectory"
} finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -ErrorAction SilentlyContinue }
    $process.Dispose()
    Write-Output "NATIVE_E2E_REPORT $reportDirectory"
}
