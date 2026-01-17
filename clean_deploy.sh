#!/bin/sh

# Exit immediately if any command fails
set -e

echo "--- 🧹 Cleaning previous builds ---"
cargo clean
#rm -rf .vercel

echo "--- 🔨 Building Rust binary ---"
# This will stop the script if compilation fails
cargo build

echo "--- 🚀 Deploying to Vercel ---"
# --yes bypasses the confirmation prompts
vercel --prod --force --yes

# vercel --prod --force --yes --archive=tgz
echo "--- ✅ Deployment complete ---"