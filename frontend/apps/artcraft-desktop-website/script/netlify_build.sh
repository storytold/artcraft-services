#!/bin/bash

set -euxo pipefail

# Runs from the Netlify base directory (= the Nx workspace root, `frontend/`).

echo "Building artcraft-desktop-website"
npx --no-install nx build @frontend/artcraft-desktop-website

echo "Final build files:"
find apps/artcraft-desktop-website/dist/
