param(
    [ValidateRange(1, 86400)][int]$DurationSeconds = 1800,
    [Parameter(Mandatory = $true)][string]$ReportDirectory
)

# 仅启动 Rust 测试宿主及其真实内核，不启动或自动操作桌面界面。
$ErrorActionPreference = 'Stop'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = 'stable'
$repository = Split-Path $PSScriptRoot -Parent
$reportRoot = [System.IO.Path]::GetFullPath($ReportDirectory)
New-Item -ItemType Directory -Path $reportRoot -Force | Out-Null
Set-Location $repository
$env:SING_BOX_TEST_BIN = Join-Path $repository 'src-tauri\resources\sing-box\windows-amd64\sing-box.exe'
$env:REQUIRE_SING_BOX_TEST_BIN = '1'
$env:ARCHITECTURE_SOAK_SECONDS = [string]$DurationSeconds
$env:ARCHITECTURE_SOAK_OUTPUT = Join-Path $reportRoot 'runtime-soak.json'
$buildOutput = Join-Path $reportRoot 'build.jsonl'
$buildErrors = Join-Path $reportRoot 'build-errors.log'
$previousPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = 'Continue'
    cargo test --manifest-path src-tauri/Cargo.toml --locked --lib --no-run --message-format=json > $buildOutput 2> $buildErrors
    $buildResult = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousPreference
}
if ($buildResult -ne 0) { throw "测试宿主编译失败，见 $buildErrors" }
$artifact = Get-Content $buildOutput | ForEach-Object { $_ | ConvertFrom-Json } |
    Where-Object { $_.reason -eq 'compiler-artifact' -and $_.executable -and $_.target.name -eq 'socks_proxy_lib' -and $_.profile.test } |
    Select-Object -Last 1
if (-not $artifact) { throw '无法定位实际 Rust 测试宿主' }
$stdout = Join-Path $reportRoot 'soak-output.log'
$stderr = Join-Path $reportRoot 'soak-errors.log'
$process = Start-Process -FilePath $artifact.executable -ArgumentList @(
    '--exact', 'runtime_soak_tests::real_core_configuration_and_transport_soak', '--ignored', '--nocapture'
) -NoNewWindow -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$null = $process.Handle # 缓存原生进程句柄，避免 Windows PowerShell 返回空 ExitCode。
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$samples = [System.Collections.Generic.List[object]]::new()
$completed = $false
try {
    while (-not $process.HasExited) {
        $process.Refresh()
        $descendants = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $($process.Id)" |
            Where-Object { $_.Name -eq 'sing-box.exe' })
        $processIds = @($process.Id) + @($descendants | ForEach-Object { $_.ProcessId })
        $usage = @(
            foreach ($processId in $processIds) {
                $item = Get-Process -Id $processId -ErrorAction SilentlyContinue
                if ($item) {
                    [ordered]@{
                        role = $(if ($processId -eq $process.Id) { 'test_host' } else { 'sing_box' })
                        pid = $processId
                        working_set_bytes = $item.WorkingSet64
                        private_bytes = $item.PrivateMemorySize64
                        handles = $item.HandleCount
                        threads = $item.Threads.Count
                        cpu_seconds = $item.TotalProcessorTime.TotalSeconds
                    }
                }
            }
        )
        $samples.Add([ordered]@{ elapsed_seconds = $timer.Elapsed.TotalSeconds; processes = $usage })
        if ($timer.Elapsed.TotalSeconds -gt $DurationSeconds + 120) { throw '稳定性测试超过预定时长与清理余量' }
        Start-Sleep -Seconds 10
    }
    $process.WaitForExit()
    $completed = $true
    [ordered]@{
        platform = 'Windows x64'
        requested_seconds = $DurationSeconds
        elapsed_seconds = $timer.Elapsed.TotalSeconds
        exit_code = $process.ExitCode
        scope = '真实内核与 ApplicationService 测试宿主；不含 GUI/WebView，不接管真实用户系统代理'
        samples = $samples
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $reportRoot 'runtime-soak-memory.json') -Encoding utf8
    if ($process.ExitCode -ne 0) { throw "真实内核稳定性测试失败，见 $stdout 和 $stderr" }
    if (-not (Test-Path $env:ARCHITECTURE_SOAK_OUTPUT)) { throw '测试没有生成成功报告' }
    Write-Output "SOAK_COMPLETED seconds=$DurationSeconds samples=$($samples.Count)"
} finally {
    if (-not $completed -and -not $process.HasExited) {
        # 超时时只结束本脚本启动的测试宿主，内核由生产 Windows Job 归属清理。
        Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
    }
    $process.Dispose()
}
