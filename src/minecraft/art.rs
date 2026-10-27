//! Selbst gezeichnete Bilder für die Minecraft-Kacheln (keine fremden Grafiken): Himmel mit
//! Leuchten, Block-Landschaft, ein Würfel und zwei Zeilen Block-Schrift ("NORISK CLIENT",
//! "MINECRAFT LAUNCHER" …). Als PNG im Cache, weil die Oberfläche aus Sicherheitsgründen nur
//! Rasterbilder lädt.

use image::{Rgba, RgbaImage};
use std::path::PathBuf;

/// Bei Änderungen am Bild erhöhen – dann wird neu gezeichnet
const VERSION: u32 = 2;

type Rgb = (u8, u8, u8);

/// Farben und Text einer Kachel
pub struct Style {
    /// Name im Cache (Dateien heißen `<key>-cover-v<VERSION>.png` …)
    pub key: &'static str,
    pub sky: [Rgb; 3],
    pub glow: Rgb,
    /// Würfel: oben, links, rechts
    pub cube: [Rgb; 3],
    pub title: &'static str,
    pub subtitle: &'static str,
    pub accent: Rgb,
}

pub const NORISK: Style = Style {
    key: "norisk",
    sky: [(11, 15, 42), (59, 29, 110), (226, 68, 92)],
    glow: (255, 90, 110),
    cube: [(255, 90, 110), (184, 36, 58), (122, 20, 48)],
    title: "NORISK",
    subtitle: "CLIENT",
    accent: (255, 210, 216),
};

/// Grasblock: oben grün, Seiten Erde
const GRASS_BLOCK: [Rgb; 3] = [(106, 190, 72), (134, 96, 67), (101, 71, 48)];

pub const JAVA: Style = Style {
    key: "mc-java",
    sky: [(14, 30, 62), (40, 96, 160), (128, 196, 232)],
    glow: (255, 226, 150),
    cube: GRASS_BLOCK,
    title: "MINECRAFT",
    subtitle: "LAUNCHER",
    accent: (206, 242, 186),
};

pub const BEDROCK: Style = Style {
    key: "mc-bedrock",
    sky: [(9, 22, 38), (22, 72, 92), (64, 156, 142)],
    glow: (120, 255, 200),
    cube: GRASS_BLOCK,
    title: "MINECRAFT",
    subtitle: "BEDROCK",
    accent: (186, 246, 222),
};

const GRASS: Rgb = (63, 163, 77);
const GRASS_DARK: Rgb = (47, 125, 58);
const DIRT: Rgb = (107, 74, 47);
const DIRT_DARK: Rgb = (88, 60, 38);
const WHITE: Rgb = (255, 255, 255);

/// Cover (600×900), Hero (1920×420), Banner (460×215) in den Cache schreiben
pub fn artwork(st: &Style) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let key = st.key;
    let dir = crate::sys::local_dir().join("art");
    std::fs::create_dir_all(&dir).ok()?;
    // Alte Fassungen aufräumen
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with(&format!("{key}-")) && !n.ends_with(&format!("-v{VERSION}.png")) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let mut out = Vec::new();
    for (name, draw) in [
        ("cover", cover as fn(&Style) -> RgbaImage),
        ("hero", hero),
        ("header", header),
    ] {
        let p = dir.join(format!("{key}-{name}-v{VERSION}.png"));
        if !p.is_file() {
            let tmp = dir.join(format!(
                "{key}-{name}-v{VERSION}.{}.tmp.png",
                std::process::id()
            ));
            draw(st)
                .save_with_format(&tmp, image::ImageFormat::Png)
                .ok()?;
            if std::fs::rename(&tmp, &p).is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
        }
        out.push(p);
    }
    let mut it = out.into_iter();
    Some((it.next()?, it.next()?, it.next()?))
}

fn px(c: Rgb) -> Rgba<u8> {
    Rgba([c.0, c.1, c.2, 255])
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// Himmel: Verlauf von oben nach unten + rötliches Leuchten um (gx, gy)
fn sky(st: &Style, w: u32, h: u32, gx: f32, gy: f32, gr: f32) -> RgbaImage {
    let (sk, glow) = (st.sky, st.glow);
    RgbaImage::from_fn(w, h, |x, y| {
        let t = y as f32 / h as f32;
        let base = if t < 0.55 {
            mix(sk[0], sk[1], t / 0.55)
        } else {
            mix(sk[1], sk[2], (t - 0.55) / 0.45)
        };
        let d = ((x as f32 - gx).powi(2) + (y as f32 - gy).powi(2)).sqrt() / gr;
        px(mix(base, glow, (1.0 - d).max(0.0) * 0.55))
    })
}

fn rect(img: &mut RgbaImage, x: i64, y: i64, w: i64, h: i64, c: Rgb) {
    let (iw, ih) = (img.width() as i64, img.height() as i64);
    for yy in y.max(0)..(y + h).min(ih) {
        for xx in x.max(0)..(x + w).min(iw) {
            img.put_pixel(xx as u32, yy as u32, px(c));
        }
    }
}

/// Block-Landschaft am unteren Rand (deterministisch)
fn terrain(img: &mut RgbaImage, size: i64, base: i64) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut seed: u32 = 0x9e37_79b9;
    for c in 0..=(w / size) {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let top = h - base - (seed % 4) as i64 * size;
        let x = c * size;
        rect(img, x, top, size, size, GRASS);
        rect(img, x, top, size, size / 4, GRASS_DARK);
        let mut y = top + size;
        let mut row = 0;
        while y < h {
            rect(
                img,
                x,
                y,
                size,
                size,
                if (c + row) % 2 == 0 { DIRT } else { DIRT_DARK },
            );
            y += size;
            row += 1;
        }
    }
}

/// Konvexes Vieleck füllen
fn polygon(img: &mut RgbaImage, pts: &[(f32, f32)], c: Rgb) {
    let (minx, maxx) = pts
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (miny, maxy) = pts
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let inside = |x: f32, y: f32| {
        let mut sign = 0.0f32;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
            if cross != 0.0 {
                if sign == 0.0 {
                    sign = cross.signum();
                } else if cross.signum() != sign {
                    return false;
                }
            }
        }
        true
    };
    for y in miny.max(0.0) as u32..(maxy.ceil() as u32).min(img.height()) {
        for x in minx.max(0.0) as u32..(maxx.ceil() as u32).min(img.width()) {
            if inside(x as f32 + 0.5, y as f32 + 0.5) {
                img.put_pixel(x, y, px(c));
            }
        }
    }
}

/// Isometrischer Würfel mit weißem Rand
fn cube(img: &mut RgbaImage, colors: [Rgb; 3], cx: f32, cy: f32, r: f32) {
    let h = r / 2.0;
    let faces = |r: f32, h: f32| {
        [
            vec![(cx, cy - r), (cx + r, cy - h), (cx, cy), (cx - r, cy - h)],
            vec![(cx - r, cy - h), (cx, cy), (cx, cy + r), (cx - r, cy + h)],
            vec![(cx + r, cy - h), (cx, cy), (cx, cy + r), (cx + r, cy + h)],
        ]
    };
    // Rand: etwas größerer weißer Würfel dahinter
    let b = r * 1.08;
    for f in faces(b, b / 2.0) {
        polygon(img, &f, WHITE);
    }
    for (f, c) in faces(r, h).iter().zip(colors) {
        polygon(img, f, c);
    }
    // Feine Kanten
    let e = (r / 40.0).max(1.0);
    polygon(
        img,
        &[
            (cx - e, cy),
            (cx + e, cy),
            (cx + e, cy + r),
            (cx - e, cy + r),
        ],
        WHITE,
    );
}

/// 5×7-Block-Schrift (nur die nötigen Buchstaben)
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        _ => [0; 7],
    }
}

fn text_width(s: &str, scale: i64) -> i64 {
    let n = s.chars().count() as i64;
    (n * 6 - 1) * scale
}

/// Text mit Schatten; `x` = linker Rand
fn text(img: &mut RgbaImage, s: &str, x: i64, y: i64, scale: i64, c: Rgb) {
    for (col, off) in [((20, 8, 30), scale / 2 + 1), (c, 0)] {
        for (i, ch) in s.chars().enumerate() {
            let gx = x + i as i64 * 6 * scale + off;
            for (row, bits) in glyph(ch).iter().enumerate() {
                for col_i in 0..5 {
                    if bits & (1 << (4 - col_i)) != 0 {
                        rect(
                            img,
                            gx + col_i * scale,
                            y + row as i64 * scale + off,
                            scale,
                            scale,
                            col,
                        );
                    }
                }
            }
        }
    }
}

fn centered(img: &mut RgbaImage, s: &str, cy: i64, scale: i64, c: Rgb) {
    let x = (img.width() as i64 - text_width(s, scale)) / 2;
    text(img, s, x, cy, scale, c);
}

/// Größte Schriftgröße bis `max`, bei der `s` in `width` Pixel passt
fn fit(s: &str, max: i64, width: i64) -> i64 {
    (1..=max)
        .rev()
        .find(|k| text_width(s, *k) <= width)
        .unwrap_or(1)
}

pub fn cover(st: &Style) -> RgbaImage {
    let mut img = sky(st, 600, 900, 300.0, 330.0, 420.0);
    terrain(&mut img, 50, 100);
    cube(&mut img, st.cube, 300.0, 320.0, 120.0);
    centered(&mut img, st.title, 490, fit(st.title, 14, 560), WHITE);
    centered(
        &mut img,
        st.subtitle,
        605,
        fit(st.subtitle, 8, 560),
        st.accent,
    );
    img
}

pub fn hero(st: &Style) -> RgbaImage {
    let mut img = sky(st, 1920, 420, 1500.0, 190.0, 600.0);
    terrain(&mut img, 48, 0);
    cube(&mut img, st.cube, 1500.0, 175.0, 95.0);
    img
}

pub fn header(st: &Style) -> RgbaImage {
    let mut img = sky(st, 460, 215, 100.0, 100.0, 220.0);
    terrain(&mut img, 24, 8);
    cube(&mut img, st.cube, 92.0, 92.0, 52.0);
    text(&mut img, st.title, 180, 62, fit(st.title, 6, 270), WHITE);
    text(
        &mut img,
        st.subtitle,
        182,
        118,
        fit(st.subtitle, 4, 270),
        st.accent,
    );
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws() {
        for st in [&NORISK, &JAVA, &BEDROCK] {
            let c = cover(st);
            assert_eq!((c.width(), c.height()), (600, 900));
            // Würfel-Oberseite in Würfelfarbe, Boden grün/braun, Himmel oben dunkel
            assert_eq!(c.get_pixel(300, 300).0[0], st.cube[0].0);
            assert!(c.get_pixel(5, 5).0[0] < 40);
            assert_ne!(*c.get_pixel(5, 895), px(st.sky[2]));
            assert_eq!((hero(st).width(), header(st).height()), (1920, 215));
            assert_eq!(hero(st).height(), 420);
        }
        assert_eq!(text_width("NORISK", 1), 35);
        // Lange Titel werden kleiner, damit sie aufs Cover passen
        assert_eq!(fit("NORISK", 14, 560), 14);
        assert!(text_width("MINECRAFT", fit("MINECRAFT", 14, 560)) <= 560);
        for ch in "NORISKCLIENTMINECRAFTLAUNCHERBEDROCK".chars() {
            assert_ne!(glyph(ch), [0; 7], "Buchstabe {ch} fehlt");
        }
    }
}
