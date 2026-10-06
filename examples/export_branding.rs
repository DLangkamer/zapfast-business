//! Regenerates raster branding assets from the canonical Business SVG.

use std::{fs, io::Write, path::Path};

const MARK: &[u8] = include_bytes!("../packaging/icons/zapfast.svg");

fn render(size: u32) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_data(MARK, &resvg::usvg::Options::default())
        .expect("valid branding SVG");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("valid image size");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().expect("encode PNG")
}

fn social_banner(width: u32, height: u32, mark_size: f32, mark_x: f32, mark_y: f32) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_data(MARK, &resvg::usvg::Options::default())
        .expect("valid branding SVG");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("valid banner size");
    pixmap.fill(resvg::tiny_skia::Color::from_rgba8(4, 18, 43, 255));

    let mut paint = resvg::tiny_skia::Paint::default();
    paint.set_color_rgba8(0, 216, 194, 34);
    if let Some(path) = resvg::tiny_skia::PathBuilder::from_circle(
        width as f32 * 0.08,
        height as f32 * 0.18,
        height as f32 * 0.62,
    ) {
        pixmap.fill_path(
            &path,
            &paint,
            resvg::tiny_skia::FillRule::Winding,
            resvg::tiny_skia::Transform::identity(),
            None,
        );
    }
    paint.set_color_rgba8(22, 142, 244, 30);
    if let Some(path) = resvg::tiny_skia::PathBuilder::from_circle(
        width as f32 * 0.78,
        height as f32 * 0.9,
        height as f32 * 0.78,
    ) {
        pixmap.fill_path(
            &path,
            &paint,
            resvg::tiny_skia::FillRule::Winding,
            resvg::tiny_skia::Transform::identity(),
            None,
        );
    }

    let scale = mark_size / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, mark_x, mark_y),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().expect("encode banner")
}

fn write_ico(path: &Path, images: &[(u32, Vec<u8>)]) {
    let count = u16::try_from(images.len()).expect("ICO image count");
    let mut file = fs::File::create(path).expect("create ICO");
    file.write_all(&0u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&count.to_le_bytes()).unwrap();

    let mut offset = 6 + images.len() * 16;
    for (size, png) in images {
        let side = if *size == 256 { 0 } else { *size as u8 };
        file.write_all(&[side, side, 0, 0]).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&32u16.to_le_bytes()).unwrap();
        file.write_all(&(png.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&(offset as u32).to_le_bytes()).unwrap();
        offset += png.len();
    }
    for (_, png) in images {
        file.write_all(png).unwrap();
    }
}

fn main() {
    let output = Path::new("branding/export");
    fs::create_dir_all(output).expect("create branding export directory");

    let sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024];
    let rendered: Vec<_> = sizes
        .into_iter()
        .map(|size| {
            let png = render(size);
            fs::write(output.join(format!("zapfast-business-{size}.png")), &png)
                .expect("write PNG");
            (size, png)
        })
        .collect();

    let ico_images: Vec<_> = rendered
        .iter()
        .filter(|(size, _)| *size <= 256)
        .cloned()
        .collect();
    write_ico(Path::new("packaging/windows/zapfast.ico"), &ico_images);
    write_ico(&output.join("zapfast-business.ico"), &ico_images);

    let png_1024 = rendered
        .iter()
        .find(|(size, _)| *size == 1024)
        .map(|(_, png)| png)
        .expect("1024 PNG");
    fs::write("packaging/macos/icon-1024.png", png_1024).expect("write macOS PNG");
    fs::write(output.join("zapfast-business-avatar.png"), png_1024).expect("write avatar");
    fs::write(
        output.join("zapfast-business-x-header-1500x500.png"),
        social_banner(1500, 500, 380.0, 560.0, 60.0),
    )
    .expect("write X header");
    fs::write(
        output.join("zapfast-business-github-social-1280x640.png"),
        social_banner(1280, 640, 480.0, 400.0, 80.0),
    )
    .expect("write GitHub social preview");
}
