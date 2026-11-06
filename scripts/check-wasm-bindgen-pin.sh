#!/usr/bin/env bash
# Ensure Dockerfile wasm-bindgen-cli matches the workspace lockfile.
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
lock_ver="$(
  awk '
    /^name = "wasm-bindgen"$/ { found=1; next }
    found && /^version = / {
      gsub(/^version = "/, "")
      gsub(/"$/, "")
      print
      exit
    }
  ' "${root}/Cargo.lock"
)"
docker_ver="$(
  sed -n 's/.*wasm-bindgen-cli.*--version \([0-9.]*\).*/\1/p' "${root}/Dockerfile" | head -1
)"

if [ -z "${lock_ver}" ] || [ -z "${docker_ver}" ]; then
  echo "check-wasm-bindgen-pin: could not parse versions (lock=${lock_ver:-?}, docker=${docker_ver:-?})" >&2
  exit 1
fi

if [ "${lock_ver}" != "${docker_ver}" ]; then
  echo "check-wasm-bindgen-pin: Dockerfile wasm-bindgen-cli ${docker_ver} != Cargo.lock wasm-bindgen ${lock_ver}" >&2
  exit 1
fi

echo "wasm-bindgen-cli pin OK (${docker_ver})"
