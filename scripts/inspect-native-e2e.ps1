param(
    [Parameter(Mandatory = $true)][string]$Application,
    [Parameter(Mandatory = $true)][string]$ProxyBeforePath,
    [switch]$TerminateOwnedCore
)

# 仅本次隔离原生应用的资源/注册表读取；崩溃注入仅限固定摘要的直接内核子进程。
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 6) {
    $env:PSModulePath = "$PSHOME\Modules;C:\Program Files\WindowsPowerShell\Modules"
}
Import-Module Microsoft.PowerShell.Utility
Import-Module Microsoft.PowerShell.Management
Import-Module CimCmdlets
Import-Module NetTCPIP
$expectedApplication = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\target\debug\socks-proxy.exe'))
if ([System.IO.Path]::GetFullPath($Application) -cne $expectedApplication) { throw 'Unexpected application path' }
$inventory = @(Get-CimInstance Win32_Process)
$roots = @($inventory | Where-Object { $_.ExecutablePath -ieq $expectedApplication })
if ($roots.Count -ne 1 -or $roots[0].SessionId -eq 0) { throw 'Require exactly one interactive isolated application' }
$root = $roots[0]
$owned = @($root)
do {
    $ids = @($owned | ForEach-Object { $_.ProcessId })
    $children = @($inventory | Where-Object { $_.ParentProcessId -in $ids -and $_.ProcessId -notin $ids })
    $owned += $children
} while ($children.Count -gt 0)
$cores = @($owned | Where-Object { $_.Name -eq 'sing-box.exe' -and $_.ParentProcessId -eq $root.ProcessId })
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
} finally { $key.Dispose() }
$baseline = (Get-Content $ProxyBeforePath -Raw -Encoding utf8).TrimStart([char]0xFEFF).Trim()
$restored = ($values | ConvertTo-Json -Depth 5 -Compress) -ceq $baseline
$port = $null
if ($cores.Count -eq 1 -and $values.ProxyEnable.value -eq 1 -and $values.ProxyServer.value -match '^127\.0\.0\.1:(\d+)$') {
    $candidate = [int]$Matches[1]
    $listener = @(Get-NetTCPConnection -LocalPort $candidate -State Listen -ErrorAction SilentlyContinue)
    if (@($listener | Where-Object { $_.OwningProcess -eq $cores[0].ProcessId }).Count -gt 0) { $port = $candidate }
}
$resources = @(
    foreach ($entry in $owned) {
        $item = Get-Process -Id $entry.ProcessId -ErrorAction SilentlyContinue
        if ($item) {
            [ordered]@{
                role = if ($entry.ProcessId -eq $root.ProcessId) { 'application' } elseif ($entry.Name -eq 'sing-box.exe') { 'core' } else { 'webview_or_child' }
                pid = $item.Id
                working_set_bytes = $item.WorkingSet64
                private_bytes = $item.PrivateMemorySize64
                handles = $item.HandleCount
                threads = $item.Threads.Count
            }
        }
    }
)
$terminated = $null
$coreHash = $null
if ($TerminateOwnedCore) {
    if ($cores.Count -ne 1 -or $null -eq $port) { throw 'Require one owned active core and its proxy listener' }
    $core = $cores[0]
    $coreHash = (Get-FileHash $core.ExecutablePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($coreHash -cne 'b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909') { throw 'Unexpected core hash' }
    $fresh = Get-CimInstance Win32_Process -Filter "ProcessId = $($core.ProcessId)"
    if ($fresh.ParentProcessId -ne $root.ProcessId -or $fresh.CreationDate -ne $core.CreationDate -or $fresh.ExecutablePath -cne $core.ExecutablePath) { throw 'Core identity changed' }
    $output = & "$env:SystemRoot\System32\taskkill.exe" /PID $core.ProcessId /F 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'Owned core termination failed' }
    $terminated = $core.ProcessId
}
$runtimeRoot = Join-Path $env:APPDATA 'com.socksproxy.desktop.architecture-e2e\runtime'
$directories = @(Get-ChildItem $runtimeRoot -Directory -ErrorAction SilentlyContinue).Count
[ordered]@{
    application_pid = $root.ProcessId
    session_id = $root.SessionId
    core_pids = @($cores | ForEach-Object { $_.ProcessId })
    proxy_port = $port
    proxy_restored = $restored
    runtime_directories = $directories
    terminated_core_pid = $terminated
    core_sha256 = $coreHash
    processes = $resources
} | ConvertTo-Json -Depth 6 -Compress
