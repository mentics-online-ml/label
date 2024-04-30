#!/bin/bash
set -e
LOCAL_DIR="$(dirname "${BASH_SOURCE[0]}")"
BASE_DIR=$(realpath "$LOCAL_DIR/..")
source "$LOCAL_DIR"/setenv.sh

if ! docker ps | grep -q scylla; then
    "$LOCAL_DIR"/scylla/run.sh
fi

cd "$BASE_DIR"

RUST_BACKTRACE=1 cargo run

# docker run -e TRADIER_API_KEY=`cat ~/.tradier_api_key` ghcr.io/mentics-online-ml/ingest:latest