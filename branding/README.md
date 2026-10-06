# ZapFast Business branding

The canonical application mark is `packaging/icons/zapfast.svg`. It preserves
ZapFast's original green disc, gradients, rim and finish, replacing only the
central chat-bubble glyph with a dark `B` for the independent Business build.

## Source files

- `zapfast-business-logo.svg`: full application mark.
- `zapfast-business-logo-small.svg`: flat small-size mark.
- `zapfast-business-tray.svg`: monochrome tray template.

## Exported files

Run `cargo run --locked --example export_branding` from the repository root.
The command refreshes:

- PNG icons at 16, 20, 24, 32, 40, 48, 64, 128, 256, 512 and 1024 pixels;
- the multiresolution Windows `.ico` used by the executable and installer;
- the 1024-pixel macOS source PNG;
- a square profile/avatar image;
- an X header at 1500 × 500;
- a GitHub social preview at 1280 × 640.

The files in `branding/export/` are ready to send with a logo-change request.
The build consumes `packaging/windows/zapfast.ico` and the SVG files under
`packaging/icons/` directly.
