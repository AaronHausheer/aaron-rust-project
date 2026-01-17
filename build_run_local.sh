#!/bin/sh
set -e

cargo clean
# rm -rf .vercel
cargo build --release
VERCEL_DEBUG=1 vercel dev --listen 8000
