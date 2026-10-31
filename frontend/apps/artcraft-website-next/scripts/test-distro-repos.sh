#!/usr/bin/env bash
# Test ArtCraft Linux package repositories using real package manager tools
# inside Ubuntu (APT), Fedora (DNF), and openSUSE (Zypper) containers.
#
# Usage:
#   ./scripts/test-distro-repos.sh [ubuntu|fedora|opensuse|all] [BASE_URL]
#
# If BASE_URL is omitted, it defaults to http://127.0.0.1:4210 and launches
# a local Next.js production server.
set -euo pipefail

TARGET_DISTRO="${1:-all}"
BASE_URL="${2:-http://127.0.0.1:4210}"

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
cleanup() {
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

test_ubuntu() {
  echo ""
  echo "=================================================="
  echo ">>> Testing Ubuntu 24.04 (APT) against $BASE_URL"
  echo "=================================================="
  $CONTAINER_TOOL run --rm --network host ubuntu:24.04 bash -c '
    set -euo pipefail
    apt-get update -qq && apt-get install -y -qq curl ca-certificates >/dev/null
    curl -fsSL "'"$BASE_URL"'/apt/artcraft.sources" -o /etc/apt/sources.list.d/artcraft.sources
    apt-get update
    apt-cache policy artcraft-launcher pdfcraft
    mkdir -p /tmp/pkg && cd /tmp/pkg
    apt-get download artcraft-launcher pdfcraft
    dpkg-deb -I artcraft-launcher*.deb | grep -E "Package:|Version:|Architecture:|Description:"
    dpkg-deb -I pdfcraft*.deb | grep -E "Package:|Version:|Architecture:|Description:"
    echo "=== Ubuntu (APT) verification passed! ==="
  '
}

test_fedora() {
  echo ""
  echo "=================================================="
  echo ">>> Testing Fedora (DNF) against $BASE_URL"
  echo "=================================================="
  $CONTAINER_TOOL run --rm --network host fedora:latest bash -c '
    set -euo pipefail
    curl -fsSL "'"$BASE_URL"'/rpm/artcraft.repo" -o /etc/yum.repos.d/artcraft.repo
    dnf --disablerepo="*" --enablerepo="artcraft" repoquery
    dnf --disablerepo="*" --enablerepo="artcraft" info artcraft-launcher pdfcraft
    mkdir -p /tmp/pkg && cd /tmp/pkg
    dnf download --disablerepo="*" --enablerepo="artcraft" artcraft-launcher pdfcraft
    rpm -qip artcraft-launcher*.rpm | grep -E "Name|Version|Architecture|Summary"
    rpm -qip pdfcraft*.rpm | grep -E "Name|Version|Architecture|Summary"
    echo "=== Fedora (DNF) verification passed! ==="
  '
}

test_opensuse() {
  echo ""
  echo "=================================================="
  echo ">>> Testing openSUSE (Zypper) against $BASE_URL"
  echo "=================================================="
  $CONTAINER_TOOL run --rm --network host opensuse/tumbleweed:latest bash -c '
    set -euo pipefail
    zypper --non-interactive addrepo --no-check --no-gpgcheck "'"$BASE_URL"'/rpm/artcraft.repo" || \
      zypper --non-interactive addrepo --no-check --no-gpgcheck "'"$BASE_URL"'/rpm" artcraft
    zypper --non-interactive --gpg-auto-import-keys refresh artcraft
    zypper --non-interactive info -r artcraft artcraft-launcher pdfcraft
    zypper --non-interactive download artcraft:artcraft-launcher artcraft:pdfcraft
    rpm -qip /var/cache/zypp/packages/artcraft/packages/artcraft-launcher*.rpm | grep -E "Name|Version|Architecture|Summary"
    rpm -qip /var/cache/zypp/packages/artcraft/packages/pdfcraft*.rpm | grep -E "Name|Version|Architecture|Summary"
    echo "=== openSUSE (Zypper) verification passed! ==="
  '
}

case "$TARGET_DISTRO" in
  ubuntu)
    test_ubuntu
    ;;
  fedora)
    test_fedora
    ;;
  opensuse)
    test_opensuse
    ;;
  all)
    test_ubuntu
    test_fedora
    test_opensuse
    ;;
  *)
    echo "Unknown distro target: $TARGET_DISTRO (expected: ubuntu, fedora, opensuse, all)" >&2
    exit 1
    ;;
esac

echo ""
echo "All requested Linux distribution package repository tests completed successfully!"
