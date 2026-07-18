param([string]$ProjectName = "sqim-protocol-stellar")

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot

Push-Location $repo
try {
    & npm --prefix app ci
    if ($LASTEXITCODE -ne 0) { throw "frontend dependency installation failed" }
    & npm run check
    if ($LASTEXITCODE -ne 0) { throw "frontend verification failed" }
    & npx vercel --prod --yes --name $ProjectName
    if ($LASTEXITCODE -ne 0) { throw "Vercel deployment failed" }
}
finally {
    Pop-Location
}
