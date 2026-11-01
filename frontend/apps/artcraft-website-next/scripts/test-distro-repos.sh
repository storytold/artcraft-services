#!/usr/bin/env bash
# Test ArtCraft Linux package repositories using real package manager tools
# inside Ubuntu (APT), Fedora (DNF), openSUSE (Zypper), and Ubuntu (Flatpak glibc) containers.
#
# Usage:
#   ./scripts/test-distro-repos.sh [ubuntu|fedora|opensuse|flatpak|all] [BASE_URL]
#
# If BASE_URL is omitted, it defaults to http://127.0.0.1:4210 and launches
# a local Next.js production server.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-all}"
BASE_URL="${2:-${NETLIFY_PREVIEW_URL:-http://127.0.0.1:4210}}"
BANNER_SEP="=================================================="
UBUNTU_IMAGE="${UBUNTU_IMAGE:-ubuntu:26.04}"
FEDORA_IMAGE="${FEDORA_IMAGE:-fedora:45}"
OPENSUSE_IMAGE="${OPENSUSE_IMAGE:-opensuse/tumbleweed:latest}"
ARCH_IMAGE="${ARCH_IMAGE:-archlinux:latest}"

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
if [[ "$BASE_URL" == "http://127.0.0.1:4210"* ]] && ! curl -sf -o /dev/null "$BASE_URL/apt/artcraft.sources" 2>/dev/null; then
  echo "Building and starting local Next.js server on 127.0.0.1:4210..."
  if [[ ! -d ".next" ]]; then
    npm run build
  fi
  npm run start -- -H 127.0.0.1 -p 4210 &
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

# Ensure openSUSE Tumbleweed image is available
ensure_opensuse_image() {
  if ! $CONTAINER_TOOL image inspect "$OPENSUSE_IMAGE" >/dev/null 2>&1; then
    echo "Pulling $OPENSUSE_IMAGE..."
    $CONTAINER_TOOL pull "$OPENSUSE_IMAGE"
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
            "Host": f"127.0.0.1:{port}",
            "x-forwarded-host": f"127.0.0.1:{port}",
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

manifest_info() {
  if [[ -x "$SCRIPT_DIR/../node_modules/.bin/jiti" ]]; then
    node --no-warnings "$SCRIPT_DIR/../node_modules/.bin/jiti" "$SCRIPT_DIR/manifest-info.mjs" "$@"
  elif command -v jiti >/dev/null 2>&1; then
    jiti "$SCRIPT_DIR/manifest-info.mjs" "$@"
  else
    echo "Error: jiti is required to execute manifest-info.mjs. Run 'npm install' first." >&2
    exit 1
  fi
}

test_ubuntu() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing Ubuntu 26.04 (APT) with $UBUNTU_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"

  local apt_pkgs apt_install_pkgs
  apt_pkgs="$(manifest_info apt-packages)"
  apt_install_pkgs="$(manifest_info apt-install-packages)"
  manifest_info apt-checksums > /tmp/expected-apt.sha256

  $CONTAINER_TOOL run --rm --network host --tmpfs /tmp/pkg:exec \
    -v /tmp/expected-apt.sha256:/tmp/expected.sha256:ro \
    "$UBUNTU_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true

    # Avoid 404s from standard Ubuntu mirrors when non-x86 architectures are added
    sed -i "/^Types: deb/a Architectures: amd64" /etc/apt/sources.list.d/ubuntu.sources 2>/dev/null || true

    apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null
    dpkg --add-architecture arm64
    dpkg --add-architecture riscv64

    curl -fsSL "'"$BASE_URL"'/apt/artcraft.sources" -o /etc/apt/sources.list.d/artcraft.sources
    apt-get update

    echo "--- Querying APT policy ---"
    apt-cache policy '"$apt_install_pkgs"'

    echo "--- Downloading all packages across all architectures in manifest ---"
    # Ensure directory is world-writable with sticky bit so unprivileged _apt user can write downloaded files
    mkdir -p /tmp/pkg && chmod 1777 /tmp/pkg && cd /tmp/pkg
    apt-get download '"$apt_pkgs"'

    echo "--- Verifying downloaded Debian packages ---"
    for deb in *.deb; do
      echo "Inspecting $deb:"
      dpkg-deb -I "$deb" | grep -E "Package:|Version:|Architecture:|Installed-Size:"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    sha256sum -c /tmp/expected.sha256

    echo "--- Testing real installation of native packages ---"
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends '"$apt_install_pkgs"'
    for pkg in '"$apt_install_pkgs"'; do
      if [[ ! -x "/usr/bin/$pkg" ]]; then
        echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2
        exit 1
      fi
      echo "  ✓ /usr/bin/$pkg verified executable"
    done

    echo "=== Ubuntu 26.04 (APT) verification and installation passed! ==="
  '
}

test_fedora() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing Fedora 45 (DNF) with $FEDORA_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"

  local dnf_pkgs dnf_install_pkgs
  dnf_pkgs="$(manifest_info dnf-packages)"
  dnf_install_pkgs="$(manifest_info dnf-install-packages)"
  manifest_info rpm-checksums > /tmp/expected-rpm.sha256

  $CONTAINER_TOOL run --rm --network host --tmpfs /tmp/pkg:exec \
    -v /tmp/expected-rpm.sha256:/tmp/expected.sha256:ro \
    "$FEDORA_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    curl -fsSL "'"$BASE_URL"'/rpm/artcraft.repo" -o /etc/yum.repos.d/artcraft.repo

    echo "--- DNF repoquery: listing all packages in repository ---"
    dnf --disablerepo="*" --enablerepo="artcraft" repoquery

    echo "--- Downloading all RPM packages across all architectures in manifest ---"
    mkdir -p /tmp/pkg && cd /tmp/pkg
    dnf download --disablerepo="*" --enablerepo="artcraft" '"$dnf_pkgs"'

    echo "--- Verifying downloaded RPM headers ---"
    for rpm_file in *.rpm; do
      echo "Inspecting $rpm_file:"
      rpm -qip "$rpm_file" | grep -E "Name|Version|Architecture|Size"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    sha256sum -c /tmp/expected.sha256

    echo "--- Testing real installation of native packages ---"
    dnf install -y --setopt=install_weak_deps=False '"$dnf_install_pkgs"'
    for pkg in '"$dnf_install_pkgs"'; do
      if [[ ! -x "/usr/bin/$pkg" ]]; then
        echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2
        exit 1
      fi
      echo "  ✓ /usr/bin/$pkg verified executable"
    done

    echo "=== Fedora 45 (DNF) verification and installation passed! ==="
  '
}

test_opensuse() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing openSUSE Tumbleweed (Zypper) with $OPENSUSE_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"
  ensure_opensuse_image

  local zypper_url="$BASE_URL"
  if [[ "$BASE_URL" == *"--"* ]]; then
    echo "Deploy preview URL contains consecutive dashes (--). Starting local forwarder to bypass libzypp URI bug..."
    start_local_forwarder "$BASE_URL" 4213
    zypper_url="http://127.0.0.1:4213"
  fi

  local zypper_install_pkgs zypper_x86_pkgs zypper_arm_pkgs zypper_riscv_pkgs
  zypper_install_pkgs="$(manifest_info zypper-install-packages)"
  zypper_x86_pkgs="$(manifest_info zypper-packages x86_64)"
  zypper_arm_pkgs="$(manifest_info zypper-packages aarch64)"
  zypper_riscv_pkgs="$(manifest_info zypper-packages riscv64)"
  manifest_info rpm-checksums > /tmp/expected-zypper.sha256

  $CONTAINER_TOOL run --rm --network host --tmpfs /tmp/pkg:exec \
    -v /tmp/expected-zypper.sha256:/tmp/expected.sha256:ro \
    "$OPENSUSE_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    mkdir -p /etc/zypp /tmp/all_pkgs

    # Disable remote mirrors to avoid downloading unrelated openSUSE package metadata
    zypper --non-interactive modifyrepo --all --disable 2>/dev/null || true

    download_for_arch() {
      local arch="$1"
      shift
      local pkgs=("$@")
      echo ""
      echo "--- Testing openSUSE Zypper for architecture: $arch ---"
      cat <<EOF > /etc/zypp/zypp.conf
[main]
arch = $arch
EOF
      zypper --non-interactive addrepo --no-check --no-gpgcheck "'"$zypper_url"'/rpm" "artcraft-$arch"
      zypper --non-interactive refresh "artcraft-$arch"
      zypper --non-interactive search -r "artcraft-$arch" -s
      zypper --non-interactive download "${pkgs[@]}"
      cp /var/cache/zypp/packages/"artcraft-$arch"/packages/*.rpm /tmp/all_pkgs/
      zypper --non-interactive removerepo "artcraft-$arch"
    }

    download_for_arch x86_64 '"$zypper_x86_pkgs"'
    download_for_arch aarch64 '"$zypper_arm_pkgs"'
    download_for_arch riscv64 '"$zypper_riscv_pkgs"'

    echo ""
    echo "--- Verifying all downloaded RPM packages ---"
    cd /tmp/all_pkgs
    for rpm_file in *.rpm; do
      echo "Inspecting $rpm_file:"
      rpm -qip "$rpm_file" | grep -E "Name|Version|Architecture|Size"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    sha256sum -c /tmp/expected.sha256

    echo "--- Testing real installation of native packages ---"
    rm -f /etc/zypp/zypp.conf
    zypper --non-interactive modifyrepo --enable repo-oss 2>/dev/null || true
    zypper --non-interactive addrepo --no-check --no-gpgcheck "'"$zypper_url"'/rpm" artcraft
    zypper --non-interactive refresh
    zypper --non-interactive install --no-recommends --allow-unsigned-rpm '"$zypper_install_pkgs"'
    for pkg in $(echo "'"$zypper_install_pkgs"'" | tr " " "\n" | sed "s/^artcraft://"); do
      if [[ ! -x "/usr/bin/$pkg" ]]; then
        echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2
        exit 1
      fi
      echo "  ✓ /usr/bin/$pkg verified executable"
    done

    echo "=== openSUSE Tumbleweed (Zypper) verification and installation passed! ==="
  '

  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
    PROXY_PID=""
  fi
}

test_flatpak() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing Flatpak (OCI Remote) against $BASE_URL"
  echo "$BANNER_SEP"

  local flatpak_url="$BASE_URL"
  if [[ "$BASE_URL" == "http://"* ]]; then
    echo "Cleartext HTTP detected. Starting local forwarder to handle Flatpak h2c upgrade..."
    start_local_forwarder "$BASE_URL" 4214
    flatpak_url="http://127.0.0.1:4214"
  fi

  local flatpak_apps
  flatpak_apps="$(manifest_info flatpak-apps)"

  # Use Ubuntu (glibc-based standard Linux container) for Flatpak tests
  $CONTAINER_TOOL run --rm --network host "$UBUNTU_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends flatpak ca-certificates curl >/dev/null

    echo "--- Adding ArtCraft Flatpak OCI remote ---"
    flatpak remote-add --user --if-not-exists test-artcraft "'"$flatpak_url"'/artcraft-oci.flatpakrepo"

    echo "--- Listing remote packages from OCI registry ---"
    OUTPUT="$(flatpak remote-ls --user -d test-artcraft)"
    echo "$OUTPUT"

    echo "--- Verifying presence of all published apps in manifest ---"
    for app in '"$flatpak_apps"'; do
      if ! echo "$OUTPUT" | grep -q "ai.storyteller.$app"; then
        echo "Error: Application ai.storyteller.$app not found in Flatpak remote!" >&2
        exit 1
      fi
      echo "  ✓ ai.storyteller.$app verified"
    done

    echo "=== Flatpak (OCI Remote) verification passed! All apps verified. ==="
  '

  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
    PROXY_PID=""
  fi
}

test_arch() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing Arch Linux (Pacman) with $ARCH_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"

  local pacman_install_pkgs
  pacman_install_pkgs="$(manifest_info pacman-install-packages)"
  manifest_info pacman-checksums > /tmp/expected-pacman.sha256

  local podman_flags=""
  if [[ "$CONTAINER_TOOL" == "podman" ]]; then
    podman_flags="--tmpfs /var/lib/pacman:exec --tmpfs /var/cache/pacman/pkg:exec"
  fi

  $CONTAINER_TOOL run --rm --network host $podman_flags \
    -v /tmp/expected-pacman.sha256:/tmp/expected.sha256:ro \
    "$ARCH_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true

    echo "--- Configuring ArtCraft repository in pacman.conf ---"
    tee -a /etc/pacman.conf << EOF

[artcraft]
SigLevel = Optional TrustAll
Server = '"$BASE_URL"'/arch/\$arch
EOF

    echo "--- Synchronizing pacman repository ---"
    pacman -Sy

    echo "--- Downloading packages to verify against manifest checksums ---"
    mkdir -p /tmp/all_pkgs && cd /tmp/all_pkgs
    for file in $(awk "{print \$2}" /tmp/expected.sha256 | grep "x86_64"); do
      echo "Fetching $file..."
      curl -fsSL "'"$BASE_URL"'/arch/x86_64/$file" -o "$file"
    done
    for file in $(awk "{print \$2}" /tmp/expected.sha256 | grep "aarch64"); do
      echo "Fetching $file..."
      curl -fsSL "'"$BASE_URL"'/arch/aarch64/$file" -o "$file"
    done

    echo "--- Verifying SHA-256 integrity digests against manifest ---"
    sha256sum -c /tmp/expected.sha256

    echo "--- Testing real installation of native packages via pacman ---"
    pacman -S --noconfirm '"$pacman_install_pkgs"'

    for pkg in '"$pacman_install_pkgs"'; do
      if [[ ! -x "/usr/bin/$pkg" ]]; then
        echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2
        exit 1
      fi
      echo "  ✓ /usr/bin/$pkg verified executable"
    done

    echo "=== Arch Linux (Pacman) verification and installation passed! ==="
  '
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
  arch)
    test_arch
    ;;
  all)
    test_ubuntu
    test_fedora
    test_opensuse
    test_flatpak
    test_arch
    ;;
  *)
    echo "Unknown target: $TARGET (expected: ubuntu, fedora, opensuse, flatpak, arch, all)" >&2
    exit 1
    ;;
esac

echo ""
echo "All requested Linux package repository tests completed successfully!"
