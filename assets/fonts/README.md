# OneOS System Fonts

## Typefaces

| Family | Styles | Usage |
|--------|--------|-------|
| **OneOS Sans** | Thin–Black (9 weights), Regular + Italic | All UI text |
| **OneOS Mono** | Regular, Medium, Bold | Terminal, code editor, monospace labels |
| **Noto Color Emoji** | – | Emoji rendering |
| **Noto CJK** | Regular, Bold | Chinese / Japanese / Korean fallback |
| **Noto Intl** | Various | Arabic, Hebrew, Devanagari, and other scripts |

## File naming convention

```
OneOSSans-<Weight>[Italic].ttf
OneOSSono-<Weight>.ttf
```

Weight names: Thin, ExtraLight, Light, Regular, Medium, SemiBold, Bold, ExtraBold, Black.

## Build integration

Font files are **not** committed to the source tree.  They are fetched from the
private font asset CDN by `build/scripts/fetch_assets.sh` during a full build and
placed under `build/images/system/fonts/` before the system image is assembled.

The `fonts.conf` file in this directory is copied verbatim to
`/system/etc/fonts.conf` in the system image.

## License

OneOS Sans and OneOS Mono are proprietary to DudasCorp.
Noto fonts are licensed under the SIL Open Font License 1.1.
