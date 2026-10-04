param([string]$ToolsDirectory)

$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = 'stable'
if (-not $ToolsDirectory) { $ToolsDirectory = Join-Path $repository '.e2e-tools' }
$toolsRoot = [System.IO.Path]::GetFullPath($ToolsDirectory)
New-Item -ItemType Directory -Path $toolsRoot -Force | Out-Null
$driver = Get-Command tauri-driver -ErrorAction SilentlyContinue
$installedPackages = (& cargo install --list) -join "`n"
if (-not $driver -or $installedPackages -notmatch '(?m)^tauri-driver v2\.1\.0:\s*$') {
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        cargo install tauri-driver --version 2.1.0 --locked
        $result = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($result -ne 0) { throw '官方 tauri-driver 2.1.0 安装失败' }
    $driver = Get-Command tauri-driver -ErrorAction Stop
}
$edge = @(
    (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),
    (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw '未找到 Windows Edge；必须使用与 WebView2 匹配的 Edge Driver' }
$edgeVersion = (Get-Item $edge).VersionInfo.ProductVersion
if ($edgeVersion -notmatch '^\d+\.\d+\.\d+\.\d+$') { throw '无法读取 Edge 版本' }
$nativeDriver = Join-Path $toolsRoot 'msedgedriver.exe'
$installed = if (Test-Path $nativeDriver) { & $nativeDriver --version } else { '' }
if ($installed -notmatch [regex]::Escape($edgeVersion)) {
    $archive = Join-Path $toolsRoot 'edgedriver-win64.zip'
    Invoke-WebRequest -UseBasicParsing -Uri "https://msedgedriver.microsoft.com/$edgeVersion/edgedriver_win64.zip" -OutFile $archive
    Expand-Archive $archive $toolsRoot -Force
}
$signature = Get-AuthenticodeSignature $nativeDriver
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') {
    throw 'Edge Driver 的 Microsoft 数字签名未通过验证'
}
$actualVersion = & $nativeDriver --version
if ($LASTEXITCODE -ne 0 -or $actualVersion -notmatch [regex]::Escape($edgeVersion)) {
    throw 'Edge Driver 与 Edge 版本不匹配'
}
[ordered]@{
    driver_version = '2.1.0'
    tauri_driver = $driver.Source
    edge_version = $edgeVersion
    native_driver = $nativeDriver
    native_driver_sha256 = (Get-FileHash $nativeDriver -Algorithm SHA256).Hash.ToLowerInvariant()
    signature = 'Microsoft Corporation / Valid'
    documentation = 'https://v2.tauri.app/develop/tests/webdriver/manual-setup/'
} | ConvertTo-Json | Set-Content (Join-Path $toolsRoot 'environment.json') -Encoding utf8
Write-Output "E2E_TOOLS_READY tauri-driver=2.1.0 edge=$edgeVersion"
