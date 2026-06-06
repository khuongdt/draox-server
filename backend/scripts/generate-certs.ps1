# Generate self-signed TLS certificates for local development.
# Output: backend/certs/server.crt and backend/certs/server.key
# Do NOT use these in production — get a certificate from a trusted CA.
#
# Requires: OpenSSL on PATH (ships with Git for Windows and many distros).
# Alternative: install via winget  →  winget install ShiningLight.OpenSSL

param(
    [int]$Days = 365,
    [string]$CommonName = "localhost"
)

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Definition
$CertsDir   = Join-Path $ScriptDir "..\certs"
$CertFile   = Join-Path $CertsDir "server.crt"
$KeyFile    = Join-Path $CertsDir "server.key"
$ExtFile    = Join-Path $env:TEMP "draox-san.cnf"

if (-not (Test-Path $CertsDir)) {
    New-Item -ItemType Directory -Path $CertsDir | Out-Null
}

# SAN config — required by modern TLS clients
@"
[req]
distinguished_name = req_dn
req_extensions     = v3_req
prompt             = no

[req_dn]
CN = $CommonName
O  = Draox-Dev
C  = US

[v3_req]
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1  = 127.0.0.1
"@ | Set-Content -Path $ExtFile -Encoding UTF8

try {
    & openssl req -x509 -newkey rsa:2048 -nodes `
        -keyout $KeyFile `
        -out    $CertFile `
        -days   $Days `
        -config $ExtFile `
        -extensions v3_req

    if ($LASTEXITCODE -ne 0) { throw "openssl exited with code $LASTEXITCODE" }

    Write-Host ""
    Write-Host "Certificates written to: $CertsDir"
    Write-Host "  cert: $CertFile"
    Write-Host "  key:  $KeyFile"
    Write-Host ""
    Write-Host "These are self-signed dev certificates and will show a browser warning."
    Write-Host "For production, replace them with certificates from a trusted CA."
} finally {
    Remove-Item -ErrorAction SilentlyContinue $ExtFile
}
