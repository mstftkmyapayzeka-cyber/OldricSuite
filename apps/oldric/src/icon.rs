//! The window's icon, drawn here: two overlapping pages, blue and red, on a dark tile. No image
//! file, so nothing to attribute.

const SIZE: u32 = 256;

/// A rounded rectangle from `(x0, y0)` to `(x1, y1)` with corner radius `r` contains `(x, y)`.
fn inside(x: f32, y: f32, (x0, y0, x1, y1): (f32, f32, f32, f32), r: f32) -> bool {
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

pub fn icon() -> egui::IconData {
    let tile = (8.0, 8.0, 248.0, 248.0);
    let back = (52.0, 40.0, 160.0, 180.0);
    let front = (100.0, 80.0, 208.0, 220.0);
    let edge = (92.0, 72.0, 216.0, 228.0);
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for py in 0..SIZE {
        for px in 0..SIZE {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            let color: [u8; 4] = if inside(x, y, front, 10.0) {
                // The front page, with three text lines.
                let line = (y - 110.0).rem_euclid(28.0) < 8.0 && x > 116.0 && x < 192.0 && y > 108.0 && y < 196.0;
                if line { [0xF6, 0xD5, 0xD1, 0xFF] } else { [0xD9, 0x4A, 0x3D, 0xFF] }
            } else if inside(x, y, edge, 16.0) {
                [0x1C, 0x20, 0x27, 0xFF]
            } else if inside(x, y, back, 10.0) {
                [0x3B, 0x6F, 0xD8, 0xFF]
            } else if inside(x, y, tile, 48.0) {
                [0x1C, 0x20, 0x27, 0xFF]
            } else {
                [0, 0, 0, 0]
            };
            rgba.extend_from_slice(&color);
        }
    }
    egui::IconData { rgba, width: SIZE, height: SIZE }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_is_a_full_square_with_clear_corners() {
        let icon = super::icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // The corner is outside the rounded tile; the middle is on a page.
        assert_eq!(icon.rgba.get(3), Some(&0));
        let middle = ((128 * 256 + 128) * 4 + 3) as usize;
        assert_eq!(icon.rgba.get(middle), Some(&0xFF));
    }
}
