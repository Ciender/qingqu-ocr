#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$SourceExe,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$utf8 = [Text.UTF8Encoding]::new($false)

function Assert-ProjectPath([string]$Candidate) {
    $absolute = [IO.Path]::GetFullPath($Candidate)
    $prefix = $projectRoot.TrimEnd([char[]]'\/') + [IO.Path]::DirectorySeparatorChar
    if (-not $absolute.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Output must stay inside the project: $absolute"
    }
    $ancestor = $absolute
    while ($ancestor -and $ancestor -ne $projectRoot) {
        if (Test-Path -LiteralPath $ancestor) {
            $item = Get-Item -LiteralPath $ancestor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Linked output paths are not supported: $ancestor"
            }
        }
        $ancestor = [IO.Path]::GetDirectoryName($ancestor)
    }
    return $absolute
}

$manifest = [IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml'))
$packageSection = [regex]::Match($manifest, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
$version = [regex]::Match($packageSection, '(?m)^version\s*=\s*"([0-9A-Za-z.+-]+)"\s*$').Groups[1].Value
if (-not $version) { throw 'Cannot read the package version from Cargo.toml.' }

if (-not $SourceExe) { $SourceExe = Join-Path $projectRoot 'target/release/qingqu-ocr-native.exe' }
$sourcePath = (Resolve-Path -LiteralPath $SourceExe).ProviderPath
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) { throw 'The release executable is missing.' }

$cargoExe = Get-Command cargo -CommandType Application | Select-Object -First 1 -ExpandProperty Source
Push-Location -LiteralPath $projectRoot
try {
    $metadataLines = & $cargoExe metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
    $metadataExit = $LASTEXITCODE
    if ($metadataExit -ne 0) { throw 'Cargo dependency metadata is unavailable. Build the project first.' }
} finally {
    Pop-Location
}
$metadata = ($metadataLines -join "`n") | ConvertFrom-Json -Depth 100
$resolved = [Collections.Generic.HashSet[string]]::new()
foreach ($node in $metadata.resolve.nodes) { $null = $resolved.Add($node.id) }
foreach ($package in $metadata.packages) {
    if (-not $package.source -or -not $resolved.Contains($package.id)) { continue }
    $noticeFolder = Join-Path $projectRoot "licenses/$($package.name)-$($package.version)"
    $noticeFiles = @(Get-ChildItem -LiteralPath $noticeFolder -File -Recurse -ErrorAction SilentlyContinue)
    if ($noticeFiles.Count -eq 0) { throw "Missing third-party license: $($package.name) $($package.version)" }
}

if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'dist' }
$outputRoot = Assert-ProjectPath $OutputDirectory
$null = New-Item -ItemType Directory -Path $outputRoot -Force
$stem = "qingqu-ocr-$version-windows-x64"
$zipPath = Assert-ProjectPath (Join-Path $outputRoot "$stem.zip")
$hashPath = Assert-ProjectPath (Join-Path $outputRoot "$stem.zip.sha256")
if ((Test-Path -LiteralPath $zipPath) -or (Test-Path -LiteralPath $hashPath)) {
    throw "Release already exists: $zipPath"
}

$entries = [Collections.Generic.List[object]]::new()
$entries.Add([pscustomobject]@{Source=$sourcePath; Target="$stem/qingqu-ocr.exe"})
foreach ($name in @('README.md', 'README.en.md', 'LICENSE', 'NOTICE.txt', 'THIRD_PARTY_NOTICES.txt', 'assets/app.svg')) {
    $file = Get-Item -LiteralPath (Join-Path $projectRoot $name)
    $entries.Add([pscustomobject]@{Source=$file.FullName; Target="$stem/$name"})
}
foreach ($folder in @('licenses', 'docs/screenshots')) {
    foreach ($file in Get-ChildItem -LiteralPath (Join-Path $projectRoot $folder) -File -Recurse) {
        if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Linked release input: $($file.FullName)" }
        $relative = [IO.Path]::GetRelativePath($projectRoot, $file.FullName).Replace('\', '/')
        $entries.Add([pscustomobject]@{Source=$file.FullName; Target="$stem/$relative"})
    }
}

$archive = $null
try {
    $archive = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
    foreach ($entry in $entries | Sort-Object Target) {
        $null = [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $archive, $entry.Source, $entry.Target, [IO.Compression.CompressionLevel]::Optimal)
    }
} catch {
    if ($archive) { $archive.Dispose(); $archive = $null }
    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath }
    throw
} finally {
    if ($archive) { $archive.Dispose() }
}
$digest = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText($hashPath, "$digest  $stem.zip`n", $utf8)
[pscustomobject]@{Archive=$zipPath; Bytes=(Get-Item -LiteralPath $zipPath).Length; SHA256=$digest; Files=$entries.Count} | ConvertTo-Json
