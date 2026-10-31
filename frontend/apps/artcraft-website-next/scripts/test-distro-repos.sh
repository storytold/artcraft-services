#!/usr/bin/env bash
# Test ArtCraft Linux package repositories using real package manager tools
# inside Ubuntu (APT), Fedora (DNF), openSUSE (Zypper), and Alpine (Flatpak) containers.
#
# Usage:
#   ./scripts/test-distro-repos.sh [ubuntu|fedora|opensuse|flatpak|all] [BASE_URL]
#
# If BASE_URL is omitted, it defaults to http://127.0.0.1:4210 and launches
# a local Next.js production server.
set -euo pipefail

TARGET="${1:-all}"
BASE_URL="${2:-${NETLIFY_PREVIEW_URL:-http://127.0.0.1:4210}}"

if [[ -d "/home/linuxbrew/.linuxbrew/bin" ]]; then
  export PATH="/home/linuxbrew/.linuxbrew/bin:$PATH"
fi

CONTAINER_TOOL=""
if command -v docker >/dev/null 2>&1; then
  CONTAINER_TOOL="docker"
elif command -v podman >/dev/null 2>&1; then
  CONTAINER_TOOL="podman"
else
  echo "Error: Neither docker nor podman found in PATH." >&2
  exit 1
fi

SERVER_PID=""
PROXY_PID=""

cleanup() {
  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$SERVER_PID" ]]; then
    echo "Stopping test Next.js server (PID $SERVER_PID)..."
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

# If testing against default local port and nothing is listening, build & start Next.js
if [[ "$BASE_URL" == "http://127.0.0.1:4210"* ]]; then
  if ! curl -sf -o /dev/null "$BASE_URL/apt/artcraft.sources" 2>/dev/null; then
    echo "Building and starting local Next.js server on 127.0.0.1:4210..."
    if [[ ! -d ".next" ]]; then
      npx next build
    fi
    npx next start -H 127.0.0.1 -p 4210 &
    SERVER_PID=$!
    
    # Wait for server to become ready
    for i in {1..30}; do
      if curl -sf -o /dev/null "$BASE_URL/apt/artcraft.sources" 2>/dev/null; then
        echo "Local Next.js server ready."
        break
      fi
      sleep 0.5
    done
  fi
fi

# Ensure opensuse:latest image is available (Docker Hub tags openSUSE as opensuse/tumbleweed)
ensure_opensuse_image() {
  if ! $CONTAINER_TOOL image inspect opensuse:latest >/dev/null 2>&1; then
    echo "Tagging opensuse/tumbleweed:latest as opensuse:latest..."
    $CONTAINER_TOOL pull opensuse/tumbleweed:latest
    $CONTAINER_TOOL tag opensuse/tumbleweed:latest opensuse:latest
  fi
}

# Start local HTTP forwarder using standard library http.client.
# Preserves 302/307 redirects to GitHub Releases and works around libzypp consecutive hyphen (--) host bug
# as well as flatpak cleartext h2c upgrade header drops.
start_local_forwarder() {
  local target_url="$1"
  local port="$2"
  
  python3 -c '
import http.server, http.client, socketserver, urllib.parse, sys

target = sys.argv[1]
port = int(sys.argv[2])
parsed = urllib.parse.urlparse(target)
host = parsed.netloc
is_ssl = parsed.scheme == "https"

class P(http.server.BaseHTTPRequestHandler):
    def handle_http(self, method):
        if is_ssl:
            conn = http.client.HTTPSConnection(host)
        else:
            conn = http.client.HTTPConnection(host)
        headers = {
            "Host": host,
            "User-Agent": self.headers.get("User-Agent", "artcraft-proxy")
        }
        for h in ("Accept", "Accept-Encoding"):
            if h in self.headers:
                headers[h] = self.headers[h]
        conn.request(method, self.path, headers=headers)
        resp = conn.getresponse()
        data = resp.read()
        self.send_response(resp.status)
        for k, v in resp.getheaders():
            if k.lower() in ("transfer-encoding", "connection", "content-length"):
                continue
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        if method != "HEAD":
            self.wfile.write(data)
        conn.close()
    def do_GET(self):
        self.handle_http("GET")
    def do_HEAD(self):
        self.handle_http("HEAD")
    def log_message(self, format, *args):
        pass

socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", port), P) as httpd:
    httpd.serve_forever()
' "$target_url" "$port" &
  PROXY_PID=$!
  sleep 1
}

test_ubuntu() {
  echo ""
  echo "=================================================="
  echo ">>> Testing Ubuntu Latest (APT) against $BASE_URL"
  echo "=================================================="
  $CONTAINER_TOOL run --rm --network host ubuntu:latest bash -c '
    set -euo pipefail

    # Avoid 404s from standard Ubuntu mirrors when non-x86 architectures are added
    sed -i "/^Types: deb/a Architectures: amd64" /etc/apt/sources.list.d/ubuntu.sources 2>/dev/null || true

    apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null
    dpkg --add-architecture arm64
    dpkg --add-architecture riscv64

    curl -fsSL "'"$BASE_URL"'/apt/artcraft.sources" -o /etc/apt/sources.list.d/artcraft.sources
    apt-get update

    echo "--- Querying APT policy ---"
    apt-cache policy artcraft-launcher pdfcraft

    echo "--- Downloading all packages across all architectures in manifest ---"
    mkdir -p /tmp/pkg && cd /tmp/pkg
    apt-get download \
      artcraft-launcher:amd64 artcraft-launcher:arm64 artcraft-launcher:riscv64 \
      pdfcraft:amd64 pdfcraft:arm64 pdfcraft:riscv64

    echo "--- Verifying downloaded Debian packages ---"
    for deb in *.deb; do
      echo "Inspecting $deb:"
      dpkg-deb -I "$deb" | grep -E "Package:|Version:|Architecture:|Installed-Size:"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    cat <<EOF > /tmp/expected.sha256
b1d4b4e93612e0a5d3f1f86452b784cfbdf14295eeb294ba9348a18d149034eb  artcraft-launcher_0.11.1-1_amd64.deb
89e3eadc69667dc117b5d7950c14311e28c4732f2691fec3b4ac9453eab6861b  artcraft-launcher_0.11.1-1_arm64.deb
0636f01916d58e7014d3f9600c7c7a185197f1ac1828880f7f63689d0ea3113e  artcraft-launcher_0.11.1-1_riscv64.deb
fb71f9b366214c8b0dafe01afba80d5280d554a7683ab7756f601a170668823b  pdfcraft_0.16.1-1_amd64.deb
7c90af4c6af71da3efb017b697163267c5930e24e4d6b041edb8be0c1e23c31e  pdfcraft_0.16.1-1_arm64.deb
853e185e3c665c6da3c9d6973a121b2b0bb745c0aaab5880b9efe4438517a826  pdfcraft_0.16.1-1_riscv64.deb
EOF
    sha256sum -c /tmp/expected.sha256

    echo "=== Ubuntu Latest (APT) verification passed! All packages verified. ==="
  '
}

test_fedora() {
  echo ""
  echo "=================================================="
  echo ">>> Testing Fedora Latest (DNF) against $BASE_URL"
  echo "=================================================="
  $CONTAINER_TOOL run --rm --network host fedora:latest bash -c '
    set -euo pipefail
    curl -fsSL "'"$BASE_URL"'/rpm/artcraft.repo" -o /etc/yum.repos.d/artcraft.repo

    echo "--- DNF repoquery: listing all packages in repository ---"
    dnf --disablerepo="*" --enablerepo="artcraft" repoquery

    echo "--- Downloading all RPM packages across all architectures in manifest ---"
    mkdir -p /tmp/pkg && cd /tmp/pkg
    dnf download --disablerepo="*" --enablerepo="artcraft" \
      artcraft-launcher.x86_64 artcraft-launcher.aarch64 artcraft-launcher.riscv64 \
      pdfcraft.x86_64 pdfcraft.aarch64 pdfcraft.riscv64

    echo "--- Verifying downloaded RPM headers ---"
    for rpm_file in *.rpm; do
      echo "Inspecting $rpm_file:"
      rpm -qip "$rpm_file" | grep -E "Name|Version|Architecture|Size"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    cat <<EOF > /tmp/expected.sha256
a86b7e9e709d7111f6b3a9d77a5cf3be8a7f0c8ee8b7486dfa42423787014b41  artcraft-launcher-0.11.1-linux-x86_64.rpm
912123716ece88e1b808c34873cfcc3871d086ec40d79bab258814d5e92a823f  artcraft-launcher-0.11.1-linux-aarch64.rpm
a172ea550f528200e2ec444353d049982a962f8347e6907122c580ed835fc53a  artcraft-launcher-0.11.1-linux-riscv64.rpm
b5d30dde0d5dea634af7a8f0a7bb1f81b75861ae7338c5b4c76a9b1da66ef2d3  pdfcraft-0.16.1-linux-x86_64.rpm
9034a6ff6e40443f700f869c357bc9a5dc4d859fe87fc4a5dcf3759d57750810  pdfcraft-0.16.1-linux-aarch64.rpm
8f1e9fa6b3da20df374d12db2bf895340530a48cd72c955e9bcfe9a458976c7e  pdfcraft-0.16.1-linux-riscv64.rpm
EOF
    sha256sum -c /tmp/expected.sha256

    echo "=== Fedora Latest (DNF) verification passed! All packages verified. ==="
  '
}

test_opensuse() {
  echo ""
  echo "=================================================="
  echo ">>> Testing openSUSE Latest (Zypper) against $BASE_URL"
  echo "=================================================="
  ensure_opensuse_image

  local zypper_url="$BASE_URL"
  if [[ "$BASE_URL" == *"--"* ]]; then
    echo "Deploy preview URL contains consecutive dashes (--). Starting local forwarder to bypass libzypp URI bug..."
    start_local_forwarder "$BASE_URL" 4213
    zypper_url="http://127.0.0.1:4213"
  fi

  $CONTAINER_TOOL run --rm --network host opensuse:latest bash -c '
    set -euo pipefail
    mkdir -p /etc/zypp /tmp/all_pkgs

    # Disable remote mirrors to avoid downloading unrelated openSUSE package metadata
    zypper --non-interactive modifyrepo --all --disable 2>/dev/null || true

    for TARGET_ARCH in x86_64 aarch64 riscv64; do
      echo ""
      echo "--- Testing openSUSE Zypper for architecture: $TARGET_ARCH ---"
      cat <<EOF > /etc/zypp/zypp.conf
[main]
arch = $TARGET_ARCH
EOF
      zypper --non-interactive addrepo --no-check --no-gpgcheck "'"$zypper_url"'/rpm" "artcraft-$TARGET_ARCH"
      zypper --non-interactive refresh "artcraft-$TARGET_ARCH"
      zypper --non-interactive search -r "artcraft-$TARGET_ARCH" -s
      zypper --non-interactive download "artcraft-$TARGET_ARCH:artcraft-launcher" "artcraft-$TARGET_ARCH:pdfcraft"
      
      cp /var/cache/zypp/packages/"artcraft-$TARGET_ARCH"/packages/*.rpm /tmp/all_pkgs/
      zypper --non-interactive removerepo "artcraft-$TARGET_ARCH"
    done

    echo ""
    echo "--- Verifying all downloaded RPM packages ---"
    cd /tmp/all_pkgs
    for rpm_file in *.rpm; do
      echo "Inspecting $rpm_file:"
      rpm -qip "$rpm_file" | grep -E "Name|Version|Architecture|Size"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    cat <<EOF > /tmp/expected.sha256
a86b7e9e709d7111f6b3a9d77a5cf3be8a7f0c8ee8b7486dfa42423787014b41  artcraft-launcher-0.11.1-linux-x86_64.rpm
912123716ece88e1b808c34873cfcc3871d086ec40d79bab258814d5e92a823f  artcraft-launcher-0.11.1-linux-aarch64.rpm
a172ea550f528200e2ec444353d049982a962f8347e6907122c580ed835fc53a  artcraft-launcher-0.11.1-linux-riscv64.rpm
b5d30dde0d5dea634af7a8f0a7bb1f81b75861ae7338c5b4c76a9b1da66ef2d3  pdfcraft-0.16.1-linux-x86_64.rpm
9034a6ff6e40443f700f869c357bc9a5dc4d859fe87fc4a5dcf3759d57750810  pdfcraft-0.16.1-linux-aarch64.rpm
8f1e9fa6b3da20df374d12db2bf895340530a48cd72c955e9bcfe9a458976c7e  pdfcraft-0.16.1-linux-riscv64.rpm
EOF
    sha256sum -c /tmp/expected.sha256

    echo "=== openSUSE Latest (Zypper) verification passed! All packages verified. ==="
  '

  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
    PROXY_PID=""
  fi
}

test_flatpak() {
  echo ""
  echo "=================================================="
  echo ">>> Testing Flatpak (OCI Remote) against $BASE_URL"
  echo "=================================================="

  local flatpak_url="$BASE_URL"
  if [[ "$BASE_URL" == "http://"* ]]; then
    echo "Cleartext HTTP detected. Starting local forwarder to handle Flatpak h2c upgrade..."
    start_local_forwarder "$BASE_URL" 4214
    flatpak_url="http://127.0.0.1:4214"
  fi

  $CONTAINER_TOOL run --rm --network host alpine:latest sh -c '
    set -euo pipefail
    apk add --no-cache flatpak >/dev/null 2>&1 || true

    echo "--- Adding ArtCraft Flatpak OCI remote ---"
    flatpak remote-add --user --if-not-exists test-artcraft "'"$flatpak_url"'/artcraft-oci.flatpakrepo"

    echo "--- Listing remote packages from OCI registry ---"
    OUTPUT="$(flatpak remote-ls --user -d test-artcraft)"
    echo "$OUTPUT"

    echo "--- Verifying presence of all 14 published apps in manifest ---"
    APPS="artcraft-launcher cadcraft deckcraft designcraft effectcraft figcraft filmcraft gridcraft lightcraft pdfcraft photocraft soundcraft vectorcraft wordcraft"
    for app in $APPS; do
      if ! echo "$OUTPUT" | grep -q "ai.storyteller.$app"; then
        echo "Error: Application ai.storyteller.$app not found in Flatpak remote!" >&2
        exit 1
      fi
      echo "  ✓ ai.storyteller.$app verified"
    done

    echo "=== Flatpak (OCI Remote) verification passed! All 14 apps verified. ==="
  '

  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
    PROXY_PID=""
  fi
}

case "$TARGET" in
  ubuntu)
    test_ubuntu
    ;;
  fedora)
    test_fedora
    ;;
  opensuse)
    test_opensuse
    ;;
  flatpak)
    test_flatpak
    ;;
  all)
    test_ubuntu
    test_fedora
    test_opensuse
    test_flatpak
    ;;
  *)
    echo "Unknown target: $TARGET (expected: ubuntu, fedora, opensuse, flatpak, all)" >&2
    exit 1
    ;;
esac

echo ""
echo "All requested Linux package repository tests completed successfully!"
