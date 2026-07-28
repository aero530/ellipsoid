<#
.SYNOPSIS
    Verify a built MSI actually installs, runs and uninstalls cleanly.

.DESCRIPTION
    Exercises the full lifecycle against the real Windows Installer, without elevation:

      1. the package declares that it does not require elevation
      2. the licence page shows the real licence and the artwork is ours
      3. a silent install succeeds as a normal user
      4. the expected files, Start Menu shortcut and PATH entry appear
      5. Windows Installer reports the product as installed
      6. the installed command-line tool runs and produces a pattern
      7. the installed GUI launches and opens its window
      8. reinstalling replaces the previous copy rather than adding a second
      9. a silent uninstall succeeds and removes everything it created

    Run after installer/build.ps1. Exits non-zero on the first failure, so it is usable as a
    release gate.

.PARAMETER Msi
    Path to the MSI. Defaults to the newest one in target/installer.

.PARAMETER KeepInstalled
    Skip the uninstall step, leaving the application installed for manual inspection.

.PARAMETER SkipGui
    Skip the check that the GUI opens a window. Useful on a machine with no usable graphics
    adapter, where the failure says nothing about the installer.
#>
[CmdletBinding()]
param(
    [string]$Msi,
    [switch]$KeepInstalled,
    [switch]$SkipGui
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$RepoDir = Split-Path -Parent $PSScriptRoot
$ProgramsDir = Join-Path $env:LOCALAPPDATA 'Programs'
$InstallDir = Join-Path $ProgramsDir 'Ellipsoid'
$Shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Ellipsoid Pattern Generator.lnk'

$script:Failures = 0

function Test-Claim($description, $condition, $detail) {
    if ($condition) {
        Write-Host "  PASS  $description" -ForegroundColor Green
    } else {
        Write-Host "  FAIL  $description" -ForegroundColor Red
        if ($detail) { Write-Host "        $detail" -ForegroundColor Red }
        $script:Failures++
    }
}

function Write-Step($message) { Write-Host "==> $message" -ForegroundColor Cyan }

<#
    Read a single value out of one of the MSI's tables.

    [void] matters on Execute and Close: InvokeMember on a void method still returns $null, and
    an unassigned expression in PowerShell is *output*. Without it the function returns three
    objects — two nulls and the value — and callers silently receive an array.
#>
function Get-MsiValue($msiPath, $sql, $property = 'StringData') {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($msiPath, 0))
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @($sql))
    [void]$view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null)
    $record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
    $value = if ($record) { $record.GetType().InvokeMember($property, 'GetProperty', $null, $record, @(1)) } else { $null }
    [void]$view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null)
    return $value
}

function Get-MsiProperty($msiPath, $property) {
    [string](Get-MsiValue $msiPath "SELECT Value FROM Property WHERE Property = '$property'")
}

<#
    Size of a stream in the MSI's Binary table.

    Used to confirm the installer carries our artwork rather than the bitmaps bundled in
    WixUIExtension. Size is enough to pin it: for an uncompressed 24-bit BMP the byte count is
    a function of the dimensions alone, so the expected values below are derived, not sampled.
#>
function Get-MsiBinarySize($msiPath, $name) {
    $size = Get-MsiValue $msiPath "SELECT Data FROM Binary WHERE Name = '$name'" 'DataSize'
    if ($null -eq $size) { return -1 }
    return [int]$size
}

<#
    Installs of *this* product, identified by UpgradeCode.

    Scoping by UpgradeCode rather than by product name matters: anything else on the machine
    with a similar name is somebody else's application, and a cleanup step must not touch it.

    This is also what Settings > Apps reads for MSI-managed installs. Per-user MSI packages do
    not write to HKCU\...\Uninstall, so looking there would wrongly report "not installed".
#>
function Get-InstalledProducts($upgradeCode) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $result = @()
    try {
        # RelatedProducts hands back a COM StringList, which PowerShell will not enumerate with
        # foreach and which indexes from zero — not the one-based indexing the rest of the
        # Windows Installer automation API uses.
        $related = $installer.GetType().InvokeMember('RelatedProducts', 'GetProperty', $null, $installer, @($upgradeCode))
        $count = $related.GetType().InvokeMember('Count', 'GetProperty', $null, $related, $null)
        for ($i = 0; $i -lt $count; $i++) {
            $code = [string]$related.GetType().InvokeMember('Item', 'GetProperty', $null, $related, @($i))
            $version = $installer.GetType().InvokeMember('ProductInfo', 'GetProperty', $null, $installer, @($code, 'VersionString'))
            $result += [pscustomobject]@{ ProductCode = $code; Version = $version }
        }
    } catch {
        # Throws rather than returning an empty list when the upgrade code is unknown.
    }
    return $result
}

function Invoke-Msi($arguments, $what) {
    $log = Join-Path $env:TEMP "ellipsoid-verify-$what.log"
    $process = Start-Process msiexec -ArgumentList ($arguments + @('/qn', '/l*v', "`"$log`"")) -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Write-Host "  msiexec $what failed with exit code $($process.ExitCode); log at $log" -ForegroundColor Red
    }
    return $process.ExitCode
}

# ---------------------------------------------------------------------------------------

if (-not $Msi) {
    $candidate = Get-ChildItem (Join-Path $RepoDir 'target\installer') -Filter '*.msi' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $candidate) { throw 'No MSI found in target/installer. Run installer/build.ps1 first.' }
    $Msi = $candidate.FullName
}
Write-Step "Verifying $Msi"

$UpgradeCode = Get-MsiProperty $Msi 'UpgradeCode'
if (-not $UpgradeCode) { throw 'The MSI has no UpgradeCode; cannot identify its installs.' }
Write-Host "  UpgradeCode $UpgradeCode" -ForegroundColor DarkGray

# A pre-existing install of *this* product would make the later assertions meaningless. Anything
# with a different UpgradeCode is somebody else's application and is left strictly alone.
# @() forces array context: a single PSCustomObject has no usable .Count in PowerShell 5.1.
$existing = @(Get-InstalledProducts $UpgradeCode)
if ($existing) {
    Write-Host "  Removing a previous install of this product first: $($existing.Version -join ', ')" -ForegroundColor DarkGray
    foreach ($product in $existing) { [void](Invoke-Msi @('/x', $product.ProductCode) 'pre-clean') }
}

Write-Step 'Package metadata'
$installer = New-Object -ComObject WindowsInstaller.Installer
$db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Msi, 0))
$summary = $db.GetType().InvokeMember('SummaryInformation', 'GetProperty', $null, $db, @(0))
$wordCount = $summary.GetType().InvokeMember('Property', 'GetProperty', $null, $summary, @(15))
# Bit 3 of the Word Count summary property means "elevated privileges are not required".
Test-Claim 'declares that elevation is not required' (($wordCount -band 8) -eq 8) "word count = $wordCount"

$licenseText = [string](Get-MsiValue $Msi "SELECT Text FROM Control WHERE Dialog_ = 'LicenseAgreementDlg' AND Control = 'LicenseText'")
Test-Claim 'the licence page shows the real licence' `
    ($licenseText -and $licenseText -match 'MIT License') `
    'the LicenseAgreementDlg text does not mention the MIT License'
Test-Claim 'the licence page is not placeholder text' `
    ($licenseText -notmatch 'Lorem ipsum') `
    'WiX substituted its bundled placeholder License.rtf - is WixUILicenseRtf set?'

# An uncompressed 24-bit BMP is 54 bytes of header plus one row per line, each row padded up to
# a 4-byte boundary: 54 + (ceil(width * 3 / 4) * 4) * height. At the sizes WixUI requires that
# is 54 + 1480*58 for the banner and 54 + 1480*312 for the panel. Checking the byte count
# therefore checks the dimensions and the pixel format together, without decoding the stream.
foreach ($art in @(@{ Name = 'WixUI_Bmp_Banner'; Size = 85894;  Dims = '493x58' },
                   @{ Name = 'WixUI_Bmp_Dialog'; Size = 461814; Dims = '493x312' })) {
    $actual = Get-MsiBinarySize $Msi $art.Name
    Test-Claim "$($art.Name) is our $($art.Dims) artwork" `
        ($actual -eq $art.Size) `
        "expected $($art.Size) bytes for a 24-bit $($art.Dims) BMP, found $actual - WiX's own bitmap, or a different size or colour depth"
}

<#
    Stand in for another application inside the shared Programs folder.

    The uninstall must leave that folder alone, but "left alone" is only observable when something
    else is in it: RemoveFolder deletes a directory only when it is empty, so on a machine where
    nothing else lives there — a CI runner, say — this install creates Programs and its uninstall
    correctly removes it again, and an unqualified Test-Path would report a failure with nothing
    wrong. A neighbour planted before the install makes the claim mean what it says everywhere,
    and turns it into a real test of the RemoveFolder behaviour the authoring relies on.
#>
$Neighbour = Join-Path $ProgramsDir 'ellipsoid-verify-neighbour'
$ProgramsPreexisted = Test-Path $ProgramsDir
[void](New-Item -ItemType Directory -Path $Neighbour -Force)
$NeighbourFile = Join-Path $Neighbour 'keep.txt'
Set-Content -Path $NeighbourFile -Encoding utf8 `
    -Value 'Placed by installer/verify.ps1 to check the uninstall leaves its neighbours alone. Safe to delete.'

Write-Step 'Silent install as a normal user'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = (New-Object Security.Principal.WindowsPrincipal($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($elevated) {
    Write-Host '  Note: running elevated, so this does not prove a non-admin install works.' -ForegroundColor Yellow
}
Test-Claim 'install succeeds' ((Invoke-Msi @('/i', "`"$Msi`"") 'install') -eq 0)

Write-Step 'Installed layout'
$appExe = Join-Path $InstallDir 'ellipsoid-app.exe'
$cliExe = Join-Path $InstallDir 'ellipsoid.exe'
Test-Claim 'ellipsoid-app.exe is installed' (Test-Path $appExe)
Test-Claim 'the ellipsoid command-line tool is installed' (Test-Path $cliExe)
Test-Claim 'LICENSE.txt travels with it' (Test-Path (Join-Path $InstallDir 'LICENSE.txt'))
Test-Claim 'README.md travels with it' (Test-Path (Join-Path $InstallDir 'README.md'))
Test-Claim 'CHANGELOG.md travels with it' (Test-Path (Join-Path $InstallDir 'CHANGELOG.md'))
Test-Claim 'installs under the user profile, not Program Files' ($InstallDir -like "$env:LOCALAPPDATA*")
Test-Claim 'Start Menu shortcut exists' (Test-Path $Shortcut)

# The user's own PATH, not the process's: the installer edits the environment in the registry and
# a running shell does not see it until it restarts.
#
# Trailing separators are trimmed on both sides before comparing. MSI resolves [INSTALLFOLDER]
# with one — the entry it writes reads `...\Programs\Ellipsoid\` — and an exact match against the
# unslashed form reports a missing PATH entry that is in fact present and working.
function Test-OnUserPath {
    $entries = [Environment]::GetEnvironmentVariable('PATH', 'User') -split ';' |
        ForEach-Object { $_.TrimEnd('\') }
    return $entries -contains $InstallDir.TrimEnd('\')
}
Test-Claim 'the install directory is on the user PATH' (Test-OnUserPath) `
    "user PATH does not contain $InstallDir"

if (Test-Path $Shortcut) {
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($Shortcut)
    Test-Claim 'shortcut points at the installed binary' ($link.TargetPath -eq $appExe) $link.TargetPath
}

$installed = @(Get-InstalledProducts $UpgradeCode)
Test-Claim 'Windows Installer reports it as installed' ($installed.Count -eq 1) "found $($installed.Count)"

Write-Step 'The installed command-line tool works'
if (Test-Path $cliExe) {
    $svg = Join-Path $env:TEMP 'ellipsoid-verify.svg'
    Remove-Item $svg -ErrorAction SilentlyContinue
    # A real pattern, not just --version: this is the one check that the shipped binary can do
    # the thing the project exists for.
    & $cliExe --a 3 --b 2 --c 2 -o $svg 2>&1 | Out-Null
    $ok = ($LASTEXITCODE -eq 0) -and (Test-Path $svg)
    Test-Claim 'generates an SVG pattern' $ok "exit code $LASTEXITCODE"
    if ($ok) {
        $head = Get-Content $svg -TotalCount 2 -ErrorAction SilentlyContinue
        Test-Claim 'the output is an SVG' (($head -join '') -match '<svg') ($head -join ' ')
    }
    Remove-Item $svg -ErrorAction SilentlyContinue
}

if ($SkipGui) {
    Write-Step 'Skipping the GUI check (-SkipGui)'
} else {
    Write-Step 'The installed application runs'
    if (Test-Path $appExe) {
        $app = Start-Process $appExe -PassThru
        $opened = $false
        try {
            Wait-Process -Id $app.Id -Timeout 20 -ErrorAction Stop
            Write-Host "        exited early with code $($app.ExitCode)" -ForegroundColor Red
        } catch {
            $opened = (Get-Process -Id $app.Id -ErrorAction SilentlyContinue).MainWindowTitle -eq 'Ellipsoid Pattern Generator'
            Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
        }
        Test-Claim 'launches and opens its window' $opened
    }
}

Write-Step 'Reinstalling replaces rather than duplicates'
Test-Claim 'reinstall succeeds' ((Invoke-Msi @('/i', "`"$Msi`"") 'reinstall') -eq 0)
Test-Claim 'still exactly one copy installed' (@(Get-InstalledProducts $UpgradeCode).Count -eq 1)

if ($KeepInstalled) {
    Write-Step 'Leaving it installed (-KeepInstalled)'
} else {
    Write-Step 'Uninstall and cleanup'
    Test-Claim 'uninstall succeeds' ((Invoke-Msi @('/x', "`"$Msi`"") 'uninstall') -eq 0)
    Test-Claim 'install directory removed' (-not (Test-Path $InstallDir))
    Test-Claim 'Start Menu shortcut removed' (-not (Test-Path $Shortcut))
    Test-Claim 'registry marker removed' (-not (Test-Path 'HKCU:\Software\Ellipsoid Pattern Generator'))
    Test-Claim 'no longer reported as installed' (@(Get-InstalledProducts $UpgradeCode).Count -eq 0)
    Test-Claim 'PATH entry removed' (-not (Test-OnUserPath)) `
        "user PATH still contains $InstallDir"
    # The parent is shared with other applications, so it and their files must survive.
    Test-Claim 'shared Programs folder left alone' (Test-Path $NeighbourFile) `
        "the neighbouring folder planted at $Neighbour did not survive the uninstall"
}

# Tidy the stand-in away, and Programs itself if this script is what created it and the uninstall
# has left it empty — which is the state a machine with no other per-user applications started in.
Remove-Item $Neighbour -Recurse -Force -ErrorAction SilentlyContinue
if (-not $ProgramsPreexisted -and (Test-Path $ProgramsDir) -and -not (Get-ChildItem $ProgramsDir -Force)) {
    Remove-Item $ProgramsDir -Force -ErrorAction SilentlyContinue
}

Write-Host ''
if ($script:Failures -gt 0) {
    Write-Host "$($script:Failures) check(s) failed." -ForegroundColor Red
    exit 1
}
Write-Host 'All installer checks passed.' -ForegroundColor Green
