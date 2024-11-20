#!/bin/bash
cargo clean
clear

# Get the directory of the script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Exit on error
set -e

echo "🏗️ Starting deployment process..."

# Create static directory in backend if it doesn't exist
echo "📁 Setting up static directory..."
rm -rf backend/static 2>/dev/null || true
mkdir -p backend/static

# Build frontend
echo "🔨 Building frontend..."
cd frontend
trunk build --release

# Ensure the build was successful
if [ ! -d "dist" ]; then
    echo "❌ Frontend build failed: dist directory not found"
    exit 1
fi

# Copy frontend files to backend/static
echo "📋 Copying static files..."
cp -r dist/* ../backend/static/

# Verify files were copied
if [ ! "$(ls -A ../backend/static)" ]; then
    echo "❌ File copy failed: backend/static is empty"
    exit 1
fi

echo "✅ Static files copied successfully"

# Move to backend and run
cd ../backend
echo "🚀 Starting Shuttle..."
shuttle deploy