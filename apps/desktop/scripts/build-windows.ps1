$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsDirectory = Join-Path $projectRoot ".tools"
$archivePath = Join-Path $toolsDirectory "strawberry-perl-5.42.2.1-64bit-portable.zip"
$perlDirectory = Join-Path $toolsDirectory "strawberry-perl"
$perlExecutable = Join-Path $perlDirectory "perl\bin\perl.exe"
$downloadUrl = "https://github.com/StrawberryPerl/Perl-Dist-Strawberry/releases/download/SP_54221_64bit/strawberry-perl-5.42.2.1-64bit-portable.zip"
$expectedSha256 = "32D83BE90CF04B807CFB9477482BC36302CDEE6F5B04CF57E81ADECBD8F07898"

New-Item -ItemType Directory -Path $toolsDirectory -Force | Out-Null
if (-not (Test-Path -LiteralPath $perlExecutable)) {
    if (-not (Test-Path -LiteralPath $archivePath)) {
        Write-Host "Baixando a ferramenta oficial necessária para compilar o SQLCipher..."
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    }
    $actualSha256 = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    if ($actualSha256 -ne $expectedSha256) {
        throw "O arquivo do Strawberry Perl não corresponde ao SHA-256 aprovado. A compilação foi interrompida."
    }
    if (Test-Path -LiteralPath $perlDirectory) {
        throw "A pasta de ferramentas está incompleta. Renomeie '.tools\strawberry-perl' e execute novamente."
    }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $perlDirectory
}

$env:PATH = "$(Join-Path $perlDirectory 'perl\bin');$(Join-Path $perlDirectory 'c\bin');$env:PATH"
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA "StockManagerPro\cargo-target"

Push-Location $projectRoot
try {
    & ".\node_modules\.bin\tauri.cmd" build
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
