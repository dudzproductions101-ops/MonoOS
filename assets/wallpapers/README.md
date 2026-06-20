# OneOS Default Wallpapers

Default wallpapers shipped with the OS.  Binary image files are downloaded
from the asset CDN during the build (`build/scripts/fetch_assets.sh`).

## Wallpaper catalogue

| ID | Name | Style | Resolution |
|----|------|-------|------------|
| `default_aurora` | Aurora | Abstract gradient | 3840×2400 |
| `default_cosmos` | Cosmos | Space photography | 3840×2400 |
| `default_minimal_dark` | Minimal Dark | Solid + subtle texture | 3840×2400 |
| `default_minimal_light` | Minimal Light | Solid + subtle texture | 3840×2400 |
| `default_oneos_brand` | OneOS Brand | Branded geometric | 3840×2400 |

## SVG preview thumbnails

Low-resolution SVG previews are included here so the wallpaper picker can
display them without loading multi-megabyte JPEGs.  They are named
`<id>_preview.svg` and rendered at 390 × 844 (portrait phone ratio).

## Format

Full-resolution wallpapers are JPEG (85 % quality) with embedded ICC profile.
A `dark` and `light` variant exists for each wallpaper where applicable,
selected automatically based on the active system theme.

## License

`default_aurora` and `default_oneos_brand` are original works by DudasCorp,
licensed CC BY-ND 4.0.  `default_cosmos` uses NASA public domain imagery.
