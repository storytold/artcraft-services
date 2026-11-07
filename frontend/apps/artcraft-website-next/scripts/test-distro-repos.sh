#!/usr/bin/env bash
# Test ArtCraft Linux package repositories using real package manager tools
# inside Ubuntu and Debian (APT), Fedora and RHEL (DNF), openSUSE (Zypper), and Ubuntu
# (Flatpak glibc) containers.
#
# Usage:
#   ./scripts/test-distro-repos.sh [ubuntu|debian|fedora|rhel|opensuse|flatpak|arch|all] BASE_URL
#
# BASE_URL is the deployment to test (a Netlify preview, a branch deploy or production); the script
# never serves the site itself, so what it verifies is what users download.
#
# Set REQUIRE_SIGNED=1 to fail instead of falling back to trusted repositories when the
# deployment publishes no signing key. CI sets it so signature verification is always exercised.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-all}"
BASE_URL="${2:-${NETLIFY_PREVIEW_URL:-}}"
if [[ -z "$BASE_URL" ]]; then
  echo "Error: pass the deployment to test, e.g. ./scripts/test-distro-repos.sh arch https://getartcraft.com" >&2
  exit 2
fi
BANNER_SEP="=================================================="
UBUNTU_IMAGE="${UBUNTU_IMAGE:-ubuntu:26.04}"
FEDORA_IMAGE="${FEDORA_IMAGE:-fedora:45}"
DEBIAN_IMAGE="${DEBIAN_IMAGE:-debian:stable-slim}"
# UBI lacks some GUI runtime libraries in its enabled repositories; AlmaLinux exercises the
# RHEL-compatible RPM/DNF package and dependency behavior with those libraries available.
RHEL_IMAGE="${RHEL_IMAGE:-almalinux:10}"
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

# A deployment with LINUX_SIGNING_KEY signs its repository metadata and advertises its key. The
# package managers below then verify signatures against that key instead of bypassing verification.
SIGNED_REPO=0
if curl -fsS -o /dev/null "$BASE_URL/keys/artcraft.asc" 2>/dev/null; then
  SIGNED_REPO=1
  echo "Repository signing is enabled at $BASE_URL; every package manager below verifies signatures."
elif [[ "${REQUIRE_SIGNED:-0}" == "1" ]]; then
  echo "Error: $BASE_URL publishes no signing key, so these tests would only exercise trusted" \
    "repositories instead of signature verification." >&2
  exit 1
else
  echo "Warning: $BASE_URL publishes no signing key, so these tests cannot exercise signature" \
    "verification and fall back to trusted repositories." >&2
fi

# libzypp reads the metadata key from repodata/repomd.xml.key rather than from the .repo file, and it
# cannot address a Netlify preview at all, so every deployment gets that route checked here.
if [[ "$SIGNED_REPO" == "1" ]]; then
  if ! curl -fsSL "$BASE_URL/rpm/repodata/repomd.xml.key" -o /tmp/artcraft-repomd.key 2>/dev/null ||
     ! grep -q "BEGIN PGP PUBLIC KEY BLOCK" /tmp/artcraft-repomd.key; then
    echo "Error: $BASE_URL serves no metadata key at /rpm/repodata/repomd.xml.key!" >&2
    exit 1
  fi
  curl -fsSL "$BASE_URL/keys/artcraft.asc" -o /tmp/artcraft-keys.asc 2>/dev/null || true
  if [[ -s /tmp/artcraft-keys.asc ]] && ! cmp -s /tmp/artcraft-repomd.key /tmp/artcraft-keys.asc; then
    echo "Error: /rpm/repodata/repomd.xml.key does not match /keys/artcraft.asc!" >&2
    exit 1
  fi
fi

# Ensure openSUSE Tumbleweed image is available
ensure_opensuse_image() {
  if ! $CONTAINER_TOOL image inspect "$OPENSUSE_IMAGE" >/dev/null 2>&1; then
    echo "Pulling $OPENSUSE_IMAGE..."
    $CONTAINER_TOOL pull "$OPENSUSE_IMAGE"
  fi
}

PROXY_PID=""

cleanup() {
  if [[ -n "$PROXY_PID" ]]; then
    kill "$PROXY_PID" >/dev/null 2>&1 || true
    wait "$PROXY_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

# libzypp rejects hostnames containing "--" ("Invalid host component") and every Netlify deploy
# preview has them, so zypper alone reaches the preview through this forwarder. It streams from the
# deployment — nothing is built or served from local files — and every other distro talks to
# BASE_URL directly.
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

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      echo "--- Installing the ArtCraft signing keyring ---"
      install -d -m 755 /etc/apt/keyrings
      curl -fsSL "'"$BASE_URL"'/keys/artcraft.gpg" -o /etc/apt/keyrings/artcraft.gpg
      chmod 644 /etc/apt/keyrings/artcraft.gpg
      [[ -s /etc/apt/keyrings/artcraft.gpg ]] || { echo "Error: the ArtCraft keyring is empty" >&2; exit 1; }
    fi

    curl -fsSL "'"$BASE_URL"'/apt/artcraft.sources" -o /etc/apt/sources.list.d/artcraft.sources
    # With Signed-By set, apt refuses metadata that is not signed by the imported key: a bad or
    # missing signature fails this update instead of being downloaded as trusted.
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

test_debian() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing Debian (APT) with $DEBIAN_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"

  local apt_pkgs apt_install_pkgs
  apt_pkgs="$(manifest_info apt-packages)"
  apt_install_pkgs="$(manifest_info apt-install-packages)"
  manifest_info apt-checksums > /tmp/expected-apt.sha256

  $CONTAINER_TOOL run --rm --network host --tmpfs /tmp/pkg:exec \
    -v /tmp/expected-apt.sha256:/tmp/expected.sha256:ro \
    "$DEBIAN_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null
    dpkg --add-architecture arm64
    dpkg --add-architecture riscv64

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      install -d -m 755 /etc/apt/keyrings
      curl -fsSL "'"$BASE_URL"'/keys/artcraft.gpg" -o /etc/apt/keyrings/artcraft.gpg
      chmod 644 /etc/apt/keyrings/artcraft.gpg
      [[ -s /etc/apt/keyrings/artcraft.gpg ]] || { echo "Error: the ArtCraft keyring is empty" >&2; exit 1; }
    fi

    curl -fsSL "'"$BASE_URL"'/apt/artcraft.sources" -o /etc/apt/sources.list.d/artcraft.sources
    apt-get update
    apt-cache policy '"$apt_install_pkgs"'

    mkdir -p /tmp/pkg && chmod 1777 /tmp/pkg && cd /tmp/pkg
    apt-get download '"$apt_pkgs"'
    for deb in *.deb; do
      dpkg-deb -I "$deb" | grep -E "Package:|Version:|Architecture:|Installed-Size:"
    done
    sha256sum -c /tmp/expected.sha256

    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends '"$apt_install_pkgs"'
    for pkg in '"$apt_install_pkgs"'; do
      [[ -x "/usr/bin/$pkg" ]] || { echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2; exit 1; }
      echo "  ✓ /usr/bin/$pkg verified executable"
    done
    echo "=== Debian (APT) verification and installation passed! ==="
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
    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      # The repository requires signed metadata. Let dnf import its own gpgkey (-y answers the
      # prompt); a manual rpm --import is not enough for that check. rpmkeys still reads the RPM
      # database, so import there as well.
      dnf -y --disablerepo="*" --enablerepo="artcraft" makecache
      rpm --import "'"$BASE_URL"'/keys/artcraft.asc"
    fi

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

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      echo "--- Verifying the OpenPGP signature inside every downloaded RPM (key imported above) ---"
      for rpm_file in *.rpm; do
        if ! rpmkeys --checksig "$rpm_file" | grep -q "signatures OK"; then
          echo "Error: $rpm_file is not signed by the ArtCraft key!" >&2
          rpmkeys --checksig "$rpm_file" >&2
          exit 1
        fi
        echo "  ✓ $rpm_file signature verified"
      done
    fi

    echo "--- Testing real installation of native packages ---"
    # artcraft.repo enables gpgcheck, so dnf verifies each package against the imported key.
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

test_rhel() {
  echo ""
  echo "$BANNER_SEP"
  echo ">>> Testing RHEL (DNF) with $RHEL_IMAGE against $BASE_URL"
  echo "$BANNER_SEP"

  local dnf_pkgs dnf_install_pkgs
  dnf_pkgs="$(manifest_info dnf-packages)"
  dnf_install_pkgs="$(manifest_info dnf-install-packages)"
  manifest_info rpm-checksums > /tmp/expected-rpm.sha256

  $CONTAINER_TOOL run --rm --network host --tmpfs /tmp/pkg:exec \
    -v /tmp/expected-rpm.sha256:/tmp/expected.sha256:ro \
    "$RHEL_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    mkdir -p /etc/yum.repos.d
    curl -fsSL "'"$BASE_URL"'/rpm/artcraft.repo" -o /etc/yum.repos.d/artcraft.repo
    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      dnf -y --disablerepo="*" --enablerepo="artcraft" makecache
      rpm --import "'"$BASE_URL"'/keys/artcraft.asc"
    fi

    dnf --disablerepo="*" --enablerepo="artcraft" repoquery
    mkdir -p /tmp/pkg && cd /tmp/pkg
    # Download the manifest RPMs from ArtCraft alone; resolve their dependencies from
    # AlmaLinux BaseOS, AppStream, CRB, and EPEL (webkit2gtk4.1) repositories during installation below.
    dnf install -y epel-release
    dnf --disablerepo="*" --enablerepo="artcraft" download '"$dnf_pkgs"'
    for rpm in *.rpm; do
      rpm -qip "$rpm" | grep -E "Name|Version|Release|Architecture|Size"
    done
    sha256sum -c /tmp/expected.sha256

    dnf --enablerepo="artcraft" install -y '"$dnf_install_pkgs"'
    for pkg in '"$dnf_install_pkgs"'; do
      [[ -x "/usr/bin/$pkg" ]] || { echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2; exit 1; }
      echo "  ✓ /usr/bin/$pkg verified executable"
    done
    echo "=== RHEL (DNF) verification and installation passed! ==="
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
    echo "libzypp cannot parse a hostname containing '--'; forwarding $BASE_URL through the local" \
      "forwarder for zypper only."
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

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      # zypper fetches the metadata signing key from <repo>/repodata/repomd.xml.key and imports it
      # when auto-import is on. Nothing is pre-imported here, so a missing key route fails the
      # refresh below instead of being masked by a key that is already trusted.
      echo "--- Refreshing with --gpg-auto-import-keys; libzypp imports repomd.xml.key itself ---"
      key_flags=(--gpg-auto-import-keys)
      repo_flags=()
      install_flags=()
    else
      key_flags=()
      repo_flags=(--no-check --no-gpgcheck)
      install_flags=(--allow-unsigned-rpm)
    fi

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
      zypper --non-interactive addrepo "${repo_flags[@]}" "'"$zypper_url"'/rpm" "artcraft-$arch"
      zypper --non-interactive "${key_flags[@]}" refresh "artcraft-$arch"
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
    zypper --non-interactive addrepo "${repo_flags[@]}" "'"$zypper_url"'/rpm" artcraft
    zypper --non-interactive "${key_flags[@]}" refresh
    zypper --non-interactive "${key_flags[@]}" install --no-recommends "${install_flags[@]}" '"$zypper_install_pkgs"'
    for pkg in $(echo "'"$zypper_install_pkgs"'" | tr " " "\n" | sed "s/^artcraft://"); do
      if [[ ! -x "/usr/bin/$pkg" ]]; then
        echo "Error: Binary /usr/bin/$pkg not found or not executable after installation!" >&2
        exit 1
      fi
      echo "  ✓ /usr/bin/$pkg verified executable"
    done

    echo "=== openSUSE Tumbleweed (Zypper) verification and installation passed! ==="
  '

  # Only the zypper leg uses the forwarder; stop it before the next distro runs.
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

  local flatpak_apps
  flatpak_apps="$(manifest_info flatpak-apps)"

  local flatpak_ids=""
  for app in $flatpak_apps; do
    if [[ "$app" == *.* ]]; then
      flatpak_ids+=" $app"
    else
      flatpak_ids+=" ai.storyteller.$app"
    fi
  done

  # Flatpak honours GPGKey from the remote descriptor, but no released flatpak takes a lookaside:
  # there is no --signature-lookaside option and a flatpakrepo file cannot carry one. So check the
  # lookaside at its source here, outside the container, where the quoting stays readable.
  if [[ "$SIGNED_REPO" == "1" ]]; then
    local descriptor lookaside registry_token digest signature_status

    echo "--- Confirming the published descriptor advertises signing ---"
    descriptor="$(curl -fsSL "$flatpak_url/artcraft-oci.flatpakrepo")"
    grep -q "^GPGKey=" <<< "$descriptor" || {
      echo "Error: the Flatpak remote descriptor publishes no GPG key!" >&2
      exit 1
    }
    lookaside="$(sed -n 's/^SignatureLookaside=//p' <<< "$descriptor")"
    [[ "$lookaside" == "$flatpak_url/oci-signatures" ]] || {
      echo "Error: the Flatpak remote descriptor has no signature lookaside (got '$lookaside')!" >&2
      exit 1
    }

    # OCI tags are per architecture, so the digest flatpak pulls is <branch>-<arch>.
    registry_token="$(curl -fsSL "https://ghcr.io/token?scope=repository:storytold/craft-anything:pull&service=ghcr.io" |
      sed -n 's/.*"token":"\([^"]*\)".*/\1/p')"
    digest="$(curl -fsSI -H "Authorization: Bearer $registry_token" \
      -H "Accept: application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json" \
      "https://ghcr.io/v2/storytold/craft-anything/manifests/stable-x86_64" |
      tr -d '\r' | sed -n 's/^[Dd]ocker-[Cc]ontent-[Dd]igest: //p')"
    [[ -n "$digest" ]] || {
      echo "Error: the registry reported no digest for storytold/craft-anything:stable-x86_64!" >&2
      exit 1
    }
    # The lookaside uses the containers/image separate-storage layout: <repo>@sha256=<hex>.
    signature_status="$(curl -s -o /tmp/artcraft-oci.sig -w '%{http_code}' \
      "$lookaside/storytold/craft-anything@${digest/:/=}/signature-1")"
    [[ "$signature_status" == "200" ]] || {
      echo "Error: the lookaside serves no signature for $digest (HTTP $signature_status)!" >&2
      exit 1
    }
    grep -q "\"docker-manifest-digest\":\"$digest\"" /tmp/artcraft-oci.sig || {
      echo "Error: the lookaside signature does not cover $digest!" >&2
      exit 1
    }
    echo "  ✓ The descriptor advertises the key and the lookaside serves a signature for $digest"
  fi

  # Use Ubuntu (glibc-based standard Linux container) for Flatpak tests
  $CONTAINER_TOOL run --rm --network host "$UBUNTU_IMAGE" bash -c '
    set -euo pipefail
    cat /etc/os-release | grep PRETTY_NAME || true
    cat /proc/sys/kernel/random/boot_id | tr -d "-" | tee /etc/machine-id >/dev/null
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends flatpak ca-certificates curl dbus >/dev/null

    echo "--- Adding ArtCraft Flatpak OCI remote ---"
    flatpak remote-add --user --if-not-exists test-artcraft "'"$flatpak_url"'/artcraft-oci.flatpakrepo"

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      # The lookaside is checked outside this container; here only the remote flatpak itself
      # stored has to ask for a key.
      echo "--- Confirming the remote verifies OCI signatures ---"
      remote_config="$HOME/.local/share/flatpak/repo/config"
      # Read the remote section with the shell: an awk program quoted inside this already
      # double-quoted container script splits at the inner quotes and never reaches awk intact.
      remote_section=""
      in_remote=0
      while IFS= read -r config_line; do
        case "$config_line" in
          "[remote \"test-artcraft\"]") in_remote=1; continue ;;
          \[*) in_remote=0; continue ;;
        esac
        if [[ "$in_remote" == "1" ]]; then
          printf -v remote_section "%s%s\n" "$remote_section" "$config_line"
        fi
      done < "$remote_config"
      grep -qx "gpg-verify=true" <<< "$remote_section" || {
        echo "Error: Flatpak remote is not configured to verify signatures!" >&2
        exit 1
      }
      echo "  ✓ Flatpak remote verifies signatures with the published key"
    fi

    echo "--- Listing remote packages from OCI registry ---"
    flatpak remote-ls --user --app --columns=application,version,branch | tee /tmp/remote-apps.txt
    cut -f1 /tmp/remote-apps.txt > /tmp/remote-ids.txt

    echo "--- Verifying presence of all published apps in manifest ---"
    for app_id in '"$flatpak_ids"'; do
      if ! grep -qxF "$app_id" /tmp/remote-ids.txt; then
        echo "Error: Application $app_id not found in Flatpak remote!" >&2
        exit 1
      fi
      echo "  ✓ $app_id verified in remote"
    done

    echo "--- Installing every published app from the OCI remote ---"
    # --no-deps: the OCI remote publishes apps only; runtimes come from Flathub. A single
    # transaction still pulls every app ref, manifest and blob, which is what this serves.
    dbus-run-session -- flatpak install --user --noninteractive --no-deps test-artcraft '"$flatpak_ids"'

    echo "--- Verifying every manifest app is installed ---"
    flatpak list --user --app --columns=application,version,branch,arch,size | tee /tmp/installed-apps.txt
    cut -f1 /tmp/installed-apps.txt > /tmp/installed-ids.txt
    missing=0
    for app_id in '"$flatpak_ids"'; do
      if grep -qxF "$app_id" /tmp/installed-ids.txt; then
        echo "  ✓ $app_id installed"
      else
        echo "Error: $app_id is not installed after the OCI install." >&2
        missing=1
      fi
    done
    [[ "$missing" -eq 0 ]] || exit 1

    echo "=== Flatpak (OCI Remote) verified: every published app installed from the registry. ==="
  '

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

    sed -i "s/^\[options\]/[options]\nDisableDownloadTimeout/" /etc/pacman.conf || true

    echo "--- Configuring ArtCraft repository in pacman.conf ---"
    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      echo "--- Importing and locally signing the ArtCraft key ---"
      # The image ships /etc/pacman.d/gnupg without a secret key, so the directory existing is not
      # enough to tell whether this keyring is usable.
      if ! gpg --homedir /etc/pacman.d/gnupg --list-secret-keys --with-colons 2>/dev/null | grep -q "^sec:"; then
        pacman-key --init
      fi
      curl -fsSL "'"$BASE_URL"'/keys/artcraft.asc" -o /tmp/artcraft.asc
      pacman-key --add /tmp/artcraft.asc
      artcraft_fingerprint="$(gpg --with-colons --import-options show-only --import /tmp/artcraft.asc | grep "^fpr:" | head -n1 | cut -d: -f10)"
      pacman-key --lsign-key "$artcraft_fingerprint"

      # Use the published configuration, which requires a signed database and signed packages.
      curl -fsSL "'"$BASE_URL"'/arch/artcraft.conf" -o /etc/pacman.d/artcraft.conf
      grep -q "SigLevel = Required DatabaseRequired" /etc/pacman.d/artcraft.conf || {
        echo "Error: the repository configuration does not require signatures!" >&2
        exit 1
      }
      if ! grep -Fxq "Include = /etc/pacman.d/artcraft.conf" /etc/pacman.conf; then
        echo "Include = /etc/pacman.d/artcraft.conf" >> /etc/pacman.conf
      fi
    else
      # Released packages are signed even when the deployment publishes no key, and pacman rejects
      # a package whose signature key is missing from the keyring even under TrustAll. An unsigned
      # deployment has nothing to verify against, so it has to disable checking outright.
      tee -a /etc/pacman.conf << EOF

[artcraft]
SigLevel = Never
Server = '"$BASE_URL"'/arch/\$arch
EOF
    fi

    echo "--- Synchronizing pacman repository ---"
    for attempt in {1..5}; do
      if curl -fsSL -m 30 -o /dev/null "'"$BASE_URL"'/arch/x86_64/artcraft.db" 2>/dev/null; then
        break
      fi
      echo "Waiting for artcraft.db to be ready (attempt $attempt/5)..."
      sleep 3
    done
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

    if [[ "'"$SIGNED_REPO"'" == "1" ]]; then
      echo "--- Verifying the detached .sig of every downloaded package with the ArtCraft key ---"
      for file in *.pkg.tar.zst; do
        case "$file" in
          *-x86_64.pkg.tar.zst) arch_dir=x86_64 ;;
          *) arch_dir=aarch64 ;;
        esac
        curl -fsSL "'"$BASE_URL"'/arch/$arch_dir/$file.sig" -o "$file.sig"
        if ! gpg --homedir /etc/pacman.d/gnupg --verify "$file.sig" "$file" >/dev/null 2>&1; then
          echo "Error: no valid ArtCraft signature for $file!" >&2
          exit 1
        fi
        echo "  ✓ $file signature verified"
      done
    fi

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
  debian)
    test_debian
    ;;
  fedora)
    test_fedora
    ;;
  rhel)
    test_rhel
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
    test_debian
    test_fedora
    test_rhel
    test_opensuse
    test_flatpak
    test_arch
    ;;
  *)
    echo "Unknown target: $TARGET (expected: ubuntu, debian, fedora, rhel, opensuse, flatpak, arch, all)" >&2
    exit 1
    ;;
esac

echo ""
echo "All requested Linux package repository tests completed successfully!"
