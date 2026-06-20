# OneOS System Images

Static image assets used by the system UI, onboarding flow, and error screens.

## Asset inventory

| File | Size | Used by |
|------|------|---------|
| `splash_logo.svg` | vector | Boot splash screen |
| `onboarding_*.svg` | vector | First-run setup wizard (5 pages) |
| `error_no_network.svg` | vector | Captive portal / offline screens |
| `error_generic.svg` | vector | Generic error illustration |
| `empty_gallery.svg` | vector | Gallery app empty state |
| `empty_files.svg` | vector | Files app empty state |

## Build integration

`build/scripts/fetch_assets.sh` downloads vendor-provided raster variants
(PNG at 1x–4x) from the asset CDN and places them alongside the SVGs in
`build/images/system/media/`.  The SVGs in this directory are the
authoritative source; the PNG rasters are derived artefacts.

## Colour usage

All SVGs use the OneOS palette CSS variables so they can be tinted at
runtime for light and dark mode:

| Variable | Light | Dark |
|----------|-------|------|
| `--color-brand` | `#4DA6FF` | `#4DA6FF` |
| `--color-surface` | `#F2F2F7` | `#1C1C1E` |
| `--color-label` | `#000000` | `#FFFFFF` |
