# OneOS System Icon Set

All icons are SVG, designed on a 24 × 24 dp grid with a 1.5 dp stroke.
Filled variants use the same grid with no stroke (solid fill).

## Naming convention

```
ic_<category>_<name>[_filled].svg
```

Examples: `ic_action_camera.svg`, `ic_action_camera_filled.svg`,
          `ic_nav_home.svg`, `ic_status_wifi_3.svg`.

## Categories

| Category | Prefix | Examples |
|----------|--------|---------|
| Actions | `ic_action_` | camera, mic, location, share, edit, delete |
| Navigation | `ic_nav_` | home, back, up, menu, search, close |
| Status | `ic_status_` | wifi_0–4, signal_0–4, battery_0–100, charging |
| Notifications | `ic_notif_` | message, call, email, alarm, download |
| Settings | `ic_settings_` | display, sound, network, privacy, security |
| Media | `ic_media_` | play, pause, stop, skip_next, skip_prev, volume |
| Files | `ic_file_` | folder, image, video, audio, document, archive |
| System | `ic_sys_` | info, warning, error, check, add, remove |

## Build integration

The full icon set (1 200+ SVGs) is fetched from the asset CDN by
`build/scripts/fetch_assets.sh`.  A build step rasterises each SVG to
PNG at 1×, 1.5×, 2×, 3×, and 4× densities and packages them into the
`res/drawable-*` directories of the system UI package.

## Color

Icons are monochromatic and use `currentColor` in SVG so they inherit the
text colour set by the parent QML/HTML element.  Tinting at runtime is
handled by QML `ColorOverlay` or CSS `filter: invert()` as appropriate.
