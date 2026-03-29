$ErrorActionPreference = "Stop"

$configUrl = "https://raw.githubusercontent.com/fisher-list/AutoDesk/main/clash-verge-config.yaml"
$configName = "clash-verge-config.yaml"

$configPaths = @(
    "$env:APPDATA\clash-verge-rev\config.yaml",
    "$env:LOCALAPPDATA\clash-verge-rev\config.yaml",
    "$env:APPDATA\clash-verge-rev\config.toml"
)

$configDir = $null
foreach ($path in $configPaths) {
    if (Test-Path (Split-Path $path -Parent)) {
        $configDir = Split-Path $path -Parent
        $configPath = $path
        break
    }
}

if (-not $configDir) {
    $configDir = "$env:APPDATA\clash-verge-rev"
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
    $configPath = "$configDir\config.yaml"
}

Write-Host "Downloading config from GitHub..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $configUrl -OutFile "$env:TEMP\$configName" -UseBasicParsing
    Copy-Item "$env:TEMP\$configName" $configPath -Force
    Write-Host "Config downloaded to: $configPath" -ForegroundColor Green
} catch {
    Write-Host "Download failed, creating config locally..." -ForegroundColor Yellow
    $configContent = @"
port: 7890
socks-port: 7891
allow-lan: true
mode: rule
log-level: info
external-controller: 127.0.0.1:9090

proxies:
  - name: "AutoDesk-US"
    type: vless
    server: 137.184.82.187
    port: 443
    uuid: a1b2c3d4-e5f6-7890-abcd-ef1234567890
    network: ws
    tls: true
    udp: true
    skip-cert-verify: true
    ws-opts:
      path: /vray
      headers:
        Host: ""

proxy-groups:
  - name: "代理"
    type: select
    proxies:
      - AutoDesk-US

rules:
  - GEOIP,CN,DIRECT
  - MATCH,代理
"@
    Set-Content -Path $configPath -Value $configContent -Encoding UTF8
    Write-Host "Config created at: $configPath" -ForegroundColor Green
}

Write-Host ""
Write-Host "Configuration complete!" -ForegroundColor Green
Write-Host "Please restart Clash Verge and select 'AutoDesk-US' node to connect." -ForegroundColor Cyan
