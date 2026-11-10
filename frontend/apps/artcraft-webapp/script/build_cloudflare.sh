#!/bin/bash

set -euxo pipefail

# The webapp bundles every workspace lib's source into one large bundle, and the
# vite-plugin-top-level-await (swc) transform over it exhausts V8's default heap on
# CI builders. Raise the heap ceiling for the build.
export NODE_OPTIONS="${NODE_OPTIONS:-} --max-old-space-size=8192"

echo "Building artcraft-webapp..."
npx nx build artcraft-webapp

echo "Ensuring Cloudflare Pages _redirects and _headers exist in dist..."
mkdir -p apps/artcraft-webapp/dist
cp -f apps/artcraft-webapp/public/_redirects apps/artcraft-webapp/dist/_redirects
cp -f apps/artcraft-webapp/public/_headers apps/artcraft-webapp/dist/_headers

echo "Build complete."
