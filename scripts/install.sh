#!/usr/bin/env bash
set -euo pipefail

REPO="${ARCANE_RMCP_REPO:-dinglebear-ai/rarcane}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${ARCANE_RMCP_VERSION:-latest}"
RELEASE_BASE_URL="${ARCANE_RMCP_RELEASE_BASE_URL:-}"
BINARY_NAME="rarcane"
INSTALLER_TMPDIR=""

cleanup_installer_tmpdir() {
  if [[ -n "${INSTALLER_TMPDIR}" ]]; then
    rm -rf -- "${INSTALLER_TMPDIR}"
  fi
}

usage() {
  cat <<'USAGE'
Install rarcane from GitHub Releases and verify its SHA-256 checksum.

Environment:
  INSTALL_DIR                    Destination directory (default: ~/.local/bin)
  ARCANE_RMCP_VERSION            Release tag (default: latest)
  ARCANE_RMCP_REPO               GitHub owner/repo
  ARCANE_RMCP_RELEASE_BASE_URL   Alternate HTTPS release base URL
USAGE
}

need() {
  command -v "$1" >/dev/null 2>&1 || { printf 'error: %s is required\n' "$1" >&2; return 1; }
}

target_asset() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "${os}:${arch}" in
    linux:x86_64|linux:amd64) printf '%s-x86_64.tar.gz' "${BINARY_NAME}" ;;
    mingw*:x86_64|msys*:x86_64|cygwin*:x86_64) printf '%s-windows-x86_64.tar.gz' "${BINARY_NAME}" ;;
    *) printf 'error: unsupported platform %s/%s\n' "${os}" "${arch}" >&2; return 1 ;;
  esac
}

validate_download_url() {
  case "$1" in
    https://*) ;;
    http://*)
      [[ "${ARCANE_RMCP_ALLOW_INSECURE_HTTP:-0}" == "1" ]] || {
        printf 'error: refusing insecure download URL: %s\n' "$1" >&2
        return 1
      }
      ;;
    *) printf 'error: download URL must use HTTPS: %s\n' "$1" >&2; return 1 ;;
  esac
}

download_file() {
  local url="$1" destination="$2"
  validate_download_url "${url}"
  if [[ "${url}" == https://* ]]; then
    curl --proto '=https' --tlsv1.2 --location --fail --silent --show-error \
      --connect-timeout 10 --max-time 120 --max-redirs 5 "${url}" -o "${destination}"
  else
    curl --proto '=http' --location --fail --silent --show-error \
      --connect-timeout 10 --max-time 120 --max-redirs 5 "${url}" -o "${destination}"
  fi
}

verify_checksum() {
  local archive="$1" checksum_file="$2" expected actual listed
  read -r expected listed < "${checksum_file}"
  [[ "${expected}" =~ ^[0-9a-fA-F]{64}$ ]] || { printf 'error: invalid checksum file\n' >&2; return 1; }
  listed="${listed#\*}"
  if [[ -n "${listed}" && "${listed}" != "$(basename "${archive}")" ]]; then
    printf 'error: checksum names unexpected asset %s\n' "${listed}" >&2
    return 1
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${archive}" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "${archive}" | awk '{print $1}')"
  else
    printf 'error: sha256sum or shasum is required\n' >&2
    return 1
  fi
  actual="$(printf '%s' "${actual}" | tr '[:upper:]' '[:lower:]')"
  expected="$(printf '%s' "${expected}" | tr '[:upper:]' '[:lower:]')"
  [[ "${actual}" == "${expected}" ]] || { printf 'error: checksum verification failed\n' >&2; return 1; }
}

extract_verified_binary() {
  local archive="$1" destination="$2" expected_entry="$3" entries listing
  entries="$(tar -tzf "${archive}")"
  [[ "${entries}" == "${expected_entry}" ]] || {
    printf 'error: archive must contain exactly %s and no paths\n' "${expected_entry}" >&2
    return 1
  }
  listing="$(tar -tvzf "${archive}")"
  [[ "${listing:0:1}" == "-" ]] || { printf 'error: archive entry is not a regular file\n' >&2; return 1; }
  tar -xzf "${archive}" -C "${destination}" --no-same-owner --no-same-permissions -- "${expected_entry}"
  [[ -f "${destination}/${expected_entry}" && ! -L "${destination}/${expected_entry}" ]] || {
    printf 'error: extracted binary is not a regular file\n' >&2
    return 1
  }
}

main() {
  if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then usage; return 0; fi
  need curl; need install; need mktemp; need tar

  local asset url expected_entry binary
  asset="$(target_asset)"
  expected_entry="${BINARY_NAME}"
  [[ "${asset}" == *windows* ]] && expected_entry="${BINARY_NAME}.exe"
  INSTALLER_TMPDIR="$(mktemp -d)"
  trap cleanup_installer_tmpdir EXIT

  if [[ -n "${RELEASE_BASE_URL}" ]]; then
    url="${RELEASE_BASE_URL%/}/${VERSION}/${asset}"
  elif [[ "${VERSION}" == "latest" ]]; then
    url="https://github.com/${REPO}/releases/latest/download/${asset}"
  else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
  fi

  mkdir -p "${INSTALL_DIR}"
  [[ -w "${INSTALL_DIR}" ]] || { printf 'error: install dir is not writable: %s\n' "${INSTALL_DIR}" >&2; return 1; }
  printf 'Downloading %s\n' "${url}" >&2
  download_file "${url}" "${INSTALLER_TMPDIR}/${asset}"
  download_file "${url}.sha256" "${INSTALLER_TMPDIR}/${asset}.sha256"
  verify_checksum "${INSTALLER_TMPDIR}/${asset}" "${INSTALLER_TMPDIR}/${asset}.sha256"
  extract_verified_binary "${INSTALLER_TMPDIR}/${asset}" "${INSTALLER_TMPDIR}" "${expected_entry}"
  binary="${INSTALLER_TMPDIR}/${expected_entry}"
  install -m 755 "${binary}" "${INSTALL_DIR}/${BINARY_NAME}"
  printf 'Installed %s to %s/%s\n' "${BINARY_NAME}" "${INSTALL_DIR}" "${BINARY_NAME}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
