#!/bin/bash

# Rift Desktop - Installation Script
# This script sets up the development environment for the Rift desktop application

set -e

echo "🚀 Rift Desktop - Installation Script"
echo "======================================"
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed!"
    echo "Please install Node.js v18 or higher from https://nodejs.org/"
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "❌ Node.js version must be 18 or higher!"
    echo "Current version: $(node -v)"
    echo "Please upgrade Node.js from https://nodejs.org/"
    exit 1
fi

echo "✅ Node.js version: $(node -v)"

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed!"
    exit 1
fi

echo "✅ npm version: $(npm -v)"
echo ""

# Install dependencies
echo "📦 Installing dependencies..."
npm install --legacy-peer-deps

if [ $? -ne 0 ]; then
    echo "❌ Failed to install dependencies!"
    exit 1
fi

echo "✅ Dependencies installed successfully!"
echo ""

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file from .env.example..."
    cp .env.example .env
    echo "✅ .env file created!"
    echo "⚠️  Please update .env with your API URL if needed"
else
    echo "ℹ️  .env file already exists"
fi

echo ""
echo "🎉 Installation complete!"
echo ""
echo "To start the application in development mode:"
echo "  npm run electron:dev"
echo ""
echo "To build the application:"
echo "  npm run electron:build"
echo ""
echo "Make sure your Rust backend API is running before starting the app!"
echo ""
