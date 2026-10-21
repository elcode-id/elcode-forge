<#
.SYNOPSIS
    ELCODE Forge (eforge) — Native Windows PowerShell Installer Script

.DESCRIPTION
    Downloads, verifies, and installs official ELCODE Forge native Windows binaries
    from GitHub Releases and registers the executable in the User PATH environment variable.

.EXAMPLE
    irm https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.ps1 | iex

.PARAMETER Version
    Target release version to install (e.g. "0.6.0", default: latest release).

.PARAMETER InstallDir
    Target directory for installed binaries (default: $env:LOCALAPPDATA\eforge\bin).
#>

[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$InstallDir = "$env:LOCALAPPDATA\eforge\bin"
)

$ErrorActionPreference = "Stop"

$RepoOwner = "elcode-id"
$RepoName = "elcode-forge"
$Target = "x86_64-pc-windows-msvc"

function Write-Info([string]$msg) {
    Write-Host "[info] $msg" -ForegroundColor Cyan
}

function Write-Success([string]$msg) {
    Write-Host " [OK] $msg" -ForegroundColor Green
}

function Write-WarningMsg([string]$msg) {
    Write-Host "[warn] $msg" -ForegroundColor Yellow
}

function Write-ErrorMsg([string]$msg) {
    Write-Host "[error] $msg" -ForegroundColor Red
}

# 1. Architecture Check
if ([IntPtr]::Size -ne 8) {
    Write-ErrorMsg "ELCODE Forge requires 64-bit Windows (x86_64). 32-bit architecture is not supported."
    exit 1
}

# 2. Resolve Target Version
if (-not $Version) {
    Write-Info "Resolving latest release version from GitHub..."
    try {
        $ReleaseUrl = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
        $Release = Invoke-RestMethod -Uri $ReleaseUrl -Headers @{ "User-Agent" = "eforge-installer" }
        $Version = $Release.tag_name
    } catch {
        $Version = "v0.6.0"
    }
}

if (-not $Version.StartsWith("v")) {
    $Version = "v$Version"
}

Write-Info "Target version: $Version"

# 3. Download URLs
$ArchiveName = "eforge-$Version-$Target.zip"
$DownloadUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$ArchiveName"
$ChecksumsUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/SHA256SUMS.txt"

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "eforge-install-$([System.Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
    # 4. Download Archive and Checksums
    $ArchivePath = Join-Path $TempDir $ArchiveName
    $ChecksumsPath = Join-Path $TempDir "SHA256SUMS.txt"

    Write-Info "Downloading $ArchiveName..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing

    Write-Info "Fetching official checksums from GitHub..."
    Invoke-WebRequest -Uri $ChecksumsUrl -OutFile $ChecksumsPath -UseBasicParsing

    # 5. Verify Cryptographic SHA256 Checksum
    $ExpectedHash = $null
    $ChecksumsLines = Get-Content -Path $ChecksumsPath
    foreach ($line in $ChecksumsLines) {
        if ($line -match "^([a-fA-F0-9]{64})\s+[*]?$([regex]::Escape($ArchiveName))$") {
            $ExpectedHash = $Matches[1].ToLower()
            break
        }
    }

    if (-not $ExpectedHash) {
        Write-ErrorMsg "Could not find checksum for '$ArchiveName' in SHA256SUMS.txt."
        exit 1
    }

    $ActualHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()

    if ($ActualHash -ne $ExpectedHash) {
        Write-ErrorMsg "SHA-256 checksum verification failed!"
        Write-ErrorMsg "Expected: $ExpectedHash"
        Write-ErrorMsg "Actual:   $ActualHash"
        exit 1
    }

    Write-Success "SHA-256 checksum verified: $ActualHash"

    # 6. Extract and Install
    $ExtractDir = Join-Path $TempDir "extracted"
    New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null
    Expand-Archive -Path $ArchivePath -DestinationPath $ExtractDir -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    # Locate and copy eforge.exe and elcode-forge.exe
    $EforgeExe = Get-ChildItem -Path $ExtractDir -Filter "eforge.exe" -Recurse | Select-Object -First 1
    if (-not $EforgeExe) {
        Write-ErrorMsg "Binary 'eforge.exe' not found inside downloaded archive."
        exit 1
    }

    Copy-Item -Path $EforgeExe.FullName -Destination (Join-Path $InstallDir "eforge.exe") -Force

    $ElcodeExe = Get-ChildItem -Path $ExtractDir -Filter "elcode-forge.exe" -Recurse | Select-Object -First 1
    if ($ElcodeExe) {
        Copy-Item -Path $ElcodeExe.FullName -Destination (Join-Path $InstallDir "elcode-forge.exe") -Force
    } else {
        Copy-Item -Path (Join-Path $InstallDir "eforge.exe") -Destination (Join-Path $InstallDir "elcode-forge.exe") -Force
    }

    Write-Success "Installed binaries to: $InstallDir"

    # 7. Register in User PATH Environment Variable
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $UserPath) {
        $UserPath = ""
    }

    $PathParts = $UserPath -split ";" | Where-Object { $_ -ne "" }
    if ($PathParts -notcontains $InstallDir) {
        $NewUserPath = "$UserPath;$InstallDir".Trim(";")
        [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
        Write-Success "Appended '$InstallDir' to User PATH."
    }

    # Update current session PATH
    if ($env:Path -split ";" -notcontains $InstallDir) {
        $env:Path = "$env:Path;$InstallDir"
    }

    # 8. Verify Installation
    $InstalledBinary = Join-Path $InstallDir "eforge.exe"
    $Output = & $InstalledBinary --version 2>&1
    Write-Success "Verified installation: $Output"

    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Green
    Write-Host " ELCODE Forge installation completed successfully!          " -ForegroundColor Green
    Write-Host "============================================================" -ForegroundColor Green
    Write-Host "Run 'eforge --help' or 'eforge doctor' to get started." -ForegroundColor Cyan

} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
