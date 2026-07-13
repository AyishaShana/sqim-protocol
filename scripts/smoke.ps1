param(
    [string]$FrontendUrl = "http://localhost:8080",
    [string]$ApiUrl = "http://localhost:8081",
    [string]$BasketId = "CC7XPFDPZEMRRHY3NJ7WPB5RDMWIXZMHNULKQALJGIWTXUXDK7JVPG4A",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot

function Step($message) {
    Write-Host "`n==> $message" -ForegroundColor Cyan
}

function Warn($message) {
    Write-Host "WARN: $message" -ForegroundColor Yellow
}

function Require-Command($name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

function Check-Http($url) {
    $response = Invoke-WebRequest -UseBasicParsing -Uri $url -TimeoutSec 10
    if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 300) {
        throw "Unexpected status $($response.StatusCode) for $url"
    }
    Write-Host "OK $($response.StatusCode) $url"
    return $response.Content
}

Push-Location $repo
try {
    Step "Repository cleanliness"
    git status --short

    Step "Frontend and API routes"
    Check-Http "$FrontendUrl/" | Out-Null
    Check-Http "$ApiUrl/health" | Out-Null
    Check-Http "$ApiUrl/baskets" | Out-Null
    Check-Http "$ApiUrl/baskets/$BasketId" | Out-Null
    Check-Http "$ApiUrl/baskets/$BasketId/metrics" | Out-Null
    Check-Http "$ApiUrl/baskets/$BasketId/history" | Out-Null

    if (-not $SkipBuild) {
        Step "Frontend tests"
        npm --prefix site test

        Step "Frontend production build"
        npm --prefix site run build

        Step "Soroban contract tests"
        cargo test -q

        Step "Soroban release WASM build"
        cargo build --release --target wasm32v1-none
    }

    Step "Optional service checks"
    if (Require-Command "go") {
        Push-Location services
        try {
            go test ./...
        } finally {
            Pop-Location
        }
    } else {
        Warn "Go is not installed; skipping services/go test ./..."
    }

    if (Require-Command "docker") {
        docker compose config --quiet
        Write-Host "OK docker compose config"
    } else {
        Warn "Docker is not installed; skipping docker compose config"
    }

    Step "Smoke complete"
} finally {
    Pop-Location
}
