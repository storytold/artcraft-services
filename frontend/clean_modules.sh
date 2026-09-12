#!/bin/bash

set -euxo pipefail

echo "Removing Build directories..."
rm -rf ./apps/artcraft-website/dist/
rm -rf ./apps/editor2d/dist/
rm -rf ./libs/api/dist/
rm -rf ./libs/build-env/dist/
rm -rf ./libs/common/dist/
rm -rf ./libs/components/action-reminder-modal/dist/
rm -rf ./libs/components/badge/dist/
rm -rf ./libs/components/button-dropdown/dist/
rm -rf ./libs/components/button-icon-select/dist/
rm -rf ./libs/components/button-icon/dist/
rm -rf ./libs/components/button-link/dist/
rm -rf ./libs/components/button/dist/
rm -rf ./libs/components/camera-settings-modal/dist/
rm -rf ./libs/components/close-button/dist/
rm -rf ./libs/components/create-3d-modal/dist/
rm -rf ./libs/components/demo-modal/dist/
rm -rf ./libs/components/file-uploader/dist/
rm -rf ./libs/components/gallery-modal/dist/
rm -rf ./libs/components/gravatar/dist/
rm -rf ./libs/components/image-input/dist/
rm -rf ./libs/components/input/dist/
rm -rf ./libs/components/label/dist/
rm -rf ./libs/components/lightbox-modal/dist/
rm -rf ./libs/components/list-dropdown/dist/
rm -rf ./libs/components/loading-spinner/dist/
rm -rf ./libs/components/loading/dist/
rm -rf ./libs/components/modal/dist/
rm -rf ./libs/components/model-selector/dist/
rm -rf ./libs/components/pagination/dist/
rm -rf ./libs/components/popover/dist/
rm -rf ./libs/components/pricing-modal/dist/
rm -rf ./libs/components/promptbox/dist/
rm -rf ./libs/components/search/dist/
rm -rf ./libs/components/select/dist/
rm -rf ./libs/components/slider-v2/dist/
rm -rf ./libs/components/switch/dist/
rm -rf ./libs/components/tab-selector/dist/
rm -rf ./libs/components/toaster/dist/
rm -rf ./libs/components/tooltip/dist/
rm -rf ./libs/components/transition-dialogue/dist/
rm -rf ./libs/components/upload-modal/dist/
rm -rf ./libs/login/dist/
rm -rf ./libs/model-list/dist/
rm -rf ./libs/soundboard/dist/
rm -rf ./libs/tauri-api/dist/
rm -rf ./libs/tauri-utils/dist/
rm -rf ./libs/analytics/google-analytics/dist/
rm -rf ./libs/state/subscription/dist/

echo "Resetting NX..."
npx nx reset

echo "Removing NX cache directory..."
rm -rf ./.nx/

echo "Removing Node Modules directories..."
rm -rf ./node_modules/

