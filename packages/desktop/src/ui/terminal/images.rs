//! Inline images, as the previous front end's xterm.js image addon showed
//! them: sixel (`DCS … q … ST`) and iTerm2's `OSC 1337 ; File=… ST`.
//!
//! alacritty does not parse either, so the byte stream is split here first;
//! everything that is not an image goes to alacritty unchanged.

use base64::Engine as _;
use image::RgbaImage;

/// Larger payloads are dropped instead of buffered without bound.
const MAX_PAYLOAD: usize = 32 * 1024 * 1024;
/// Largest decoded image side, in pixels.
const MAX_SIDE: usize = 8192;
const ITERM_PREFIX: &[u8] = b"\x1b]1337;File=";

/// One piece of terminal output.
#[derive(Debug, PartialEq, Eq)]
pub enum Chunk {
    Text(Vec<u8>),
    /// The whole DCS, from `ESC P` up to (not including) the terminator.
    Sixel(Vec<u8>),
    /// What follows `File=`: `args:base64`.
    Iterm(Vec<u8>),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Escape,
    DcsParams,
    Osc,
    Sixel,
    SixelEscape,
    Iterm,
    ItermEscape,
    /// An oversized image: skip it up to its terminator.
    Discard,
    DiscardEscape,
}

/// Splits output into text and image sequences, across chunk boundaries.
#[derive(Default)]
pub struct Scanner {
    state: State,
    /// Bytes of the sequence being recognised or collected.
    pending: Vec<u8>,
}

impl Scanner {
    pub fn scan(&mut self, bytes: &[u8]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut text = Vec::new();
        for &byte in bytes {
            self.step(byte, &mut text, &mut chunks);
        }
        // Text is handed on right away; only a possible image start waits.
        if !text.is_empty() {
            chunks.push(Chunk::Text(text));
        }
        chunks
    }

    fn step(&mut self, byte: u8, text: &mut Vec<u8>, chunks: &mut Vec<Chunk>) {
        fn emit(chunk: Chunk, text: &mut Vec<u8>, chunks: &mut Vec<Chunk>) {
            if !text.is_empty() {
                chunks.push(Chunk::Text(std::mem::take(text)));
            }
            chunks.push(chunk);
        }
        match self.state {
            State::Ground => {
                if byte == 0x1b {
                    self.state = State::Escape;
                    self.pending = vec![byte];
                } else {
                    text.push(byte);
                }
            }
            State::Escape => {
                self.pending.push(byte);
                self.state = match byte {
                    b'P' => State::DcsParams,
                    b']' => State::Osc,
                    _ => {
                        self.pass_through(text);
                        return;
                    }
                };
            }
            State::DcsParams => {
                self.pending.push(byte);
                match byte {
                    b'0'..=b'9' | b';' if self.pending.len() < 32 => {}
                    b'q' => self.state = State::Sixel,
                    _ => self.pass_through(text),
                }
            }
            State::Osc => {
                self.pending.push(byte);
                if self.pending.len() == ITERM_PREFIX.len() && self.pending == ITERM_PREFIX {
                    self.state = State::Iterm;
                    self.pending.clear();
                } else if !ITERM_PREFIX.starts_with(&self.pending) {
                    self.pass_through(text);
                }
            }
            State::Sixel | State::Iterm => {
                let sixel = self.state == State::Sixel;
                match byte {
                    0x1b => {
                        self.state = if sixel {
                            State::SixelEscape
                        } else {
                            State::ItermEscape
                        }
                    }
                    0x07 if !sixel => {
                        emit(
                            Chunk::Iterm(std::mem::take(&mut self.pending)),
                            text,
                            chunks,
                        );
                        self.state = State::Ground;
                    }
                    // CAN and SUB cancel a control string.
                    0x18 | 0x1a => {
                        self.pending.clear();
                        self.state = State::Ground;
                    }
                    _ if self.pending.len() >= MAX_PAYLOAD => {
                        self.pending = Vec::new();
                        self.state = State::Discard;
                    }
                    _ => self.pending.push(byte),
                }
            }
            State::SixelEscape | State::ItermEscape => {
                let payload = std::mem::take(&mut self.pending);
                emit(
                    if self.state == State::SixelEscape {
                        Chunk::Sixel(payload)
                    } else {
                        Chunk::Iterm(payload)
                    },
                    text,
                    chunks,
                );
                self.state = State::Ground;
                // ESC ends the string; anything but `\` starts a new sequence.
                if byte != b'\\' {
                    self.step(0x1b, text, chunks);
                    self.step(byte, text, chunks);
                }
            }
            State::Discard => match byte {
                0x1b => self.state = State::DiscardEscape,
                0x07 | 0x18 | 0x1a => self.state = State::Ground,
                _ => {}
            },
            State::DiscardEscape => {
                self.state = State::Ground;
                if byte != b'\\' {
                    self.step(0x1b, text, chunks);
                    self.step(byte, text, chunks);
                }
            }
        }
    }

    /// Not an image after all: the collected bytes are ordinary output. The
    /// last one may itself start a sequence.
    fn pass_through(&mut self, text: &mut Vec<u8>) {
        let mut pending = std::mem::take(&mut self.pending);
        self.state = State::Ground;
        let last = pending.pop();
        text.extend(pending);
        if let Some(last) = last {
            if last == 0x1b {
                self.state = State::Escape;
                self.pending = vec![last];
            } else {
                text.push(last);
            }
        }
    }
}

// --- Sixel ------------------------------------------------------------------

/// The VT340's default palette, which sixel images assume.
const VT340: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (51, 51, 204),
    (204, 33, 33),
    (51, 204, 51),
    (204, 51, 204),
    (51, 204, 204),
    (204, 204, 51),
    (120, 120, 120),
    (69, 69, 69),
    (87, 87, 153),
    (153, 69, 69),
    (87, 153, 87),
    (153, 87, 153),
    (87, 153, 153),
    (153, 153, 87),
    (204, 204, 204),
];

fn percent(value: u32) -> u8 {
    (value.min(100) * 255 / 100) as u8
}

/// DEC HLS puts blue at 0°, red at 120° and green at 240°.
fn hls(hue: u32, lightness: u32, saturation: u32) -> (u8, u8, u8) {
    let h = ((hue + 240) % 360) as f32 / 360.;
    let l = lightness.min(100) as f32 / 100.;
    let s = saturation.min(100) as f32 / 100.;
    if s == 0. {
        let v = (l * 255.) as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 { l * (1. + s) } else { l + s - l * s };
    let p = 2. * l - q;
    let channel = |mut t: f32| {
        t = t.rem_euclid(1.);
        let v = if t < 1. / 6. {
            p + (q - p) * 6. * t
        } else if t < 0.5 {
            q
        } else if t < 2. / 3. {
            p + (q - p) * (2. / 3. - t) * 6.
        } else {
            p
        };
        (v * 255.).round() as u8
    };
    (channel(h + 1. / 3.), channel(h), channel(h - 1. / 3.))
}

/// Decodes a sixel DCS (`ESC P params q data`). Unset pixels stay transparent.
pub fn decode_sixel(sequence: &[u8]) -> Option<RgbaImage> {
    let start = sequence.iter().position(|&byte| byte == b'q')? + 1;
    let data = &sequence[start..];
    let mut palette = [[0u8; 4]; 256];
    for (index, &(r, g, b)) in VT340.iter().enumerate() {
        palette[index] = [r, g, b, 255];
    }
    let mut rows: Vec<Vec<[u8; 4]>> = Vec::new();
    let (mut x, mut y, mut color, mut width) = (0usize, 0usize, 0usize, 0usize);
    let mut index = 0;
    let number = |index: &mut usize| -> Option<u32> {
        let begin = *index;
        let mut value: u32 = 0;
        while let Some(&digit @ b'0'..=b'9') = data.get(*index) {
            value = value
                .saturating_mul(10)
                .saturating_add(u32::from(digit - b'0'));
            *index += 1;
        }
        (*index > begin).then_some(value)
    };
    let numbers = |index: &mut usize| -> Vec<u32> {
        let mut values = vec![number(index).unwrap_or(0)];
        while data.get(*index) == Some(&b';') {
            *index += 1;
            values.push(number(index).unwrap_or(0));
        }
        values
    };
    while index < data.len() {
        let byte = data[index];
        index += 1;
        let mut repeat = 1;
        let bits = match byte {
            b'"' => {
                numbers(&mut index);
                continue;
            }
            b'#' => {
                let values = numbers(&mut index);
                color = values[0] as usize % 256;
                if let [_, space, a, b, c, ..] = values[..] {
                    let (r, g, bl) = match space {
                        1 => hls(a, b, c),
                        _ => (percent(a), percent(b), percent(c)),
                    };
                    palette[color] = [r, g, bl, 255];
                }
                continue;
            }
            b'$' => {
                x = 0;
                continue;
            }
            b'-' => {
                x = 0;
                y += 6;
                continue;
            }
            b'!' => {
                repeat = number(&mut index).unwrap_or(1).max(1) as usize;
                match data.get(index) {
                    Some(&sixel @ 0x3f..=0x7e) => {
                        index += 1;
                        sixel - 0x3f
                    }
                    _ => continue,
                }
            }
            0x3f..=0x7e => byte - 0x3f,
            _ => continue,
        };
        let end = (x + repeat).min(MAX_SIDE);
        if bits != 0 {
            for bit in 0..6 {
                if bits & (1 << bit) == 0 || y + bit >= MAX_SIDE {
                    continue;
                }
                let row_index = y + bit;
                if rows.len() <= row_index {
                    rows.resize(row_index + 1, Vec::new());
                }
                let row = &mut rows[row_index];
                if row.len() < end {
                    row.resize(end, [0; 4]);
                }
                row[x..end].fill(palette[color]);
            }
        }
        x = end;
        width = width.max(x);
    }
    let height = rows.len();
    if width == 0 || height == 0 {
        return None;
    }
    let mut image = RgbaImage::new(width as u32, height as u32);
    for (row_index, row) in rows.iter().enumerate() {
        for (column, pixel) in row.iter().enumerate() {
            image.put_pixel(column as u32, row_index as u32, image::Rgba(*pixel));
        }
    }
    Some(image)
}

// --- iTerm2 -----------------------------------------------------------------

/// How big an iTerm2 image asks to be shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    Auto,
    Cells(f32),
    Pixels(f32),
    Percent(f32),
}

impl Extent {
    fn parse(value: &str) -> Self {
        let number = |text: &str| text.trim().parse::<f32>().ok().filter(|n| *n > 0.);
        if let Some(pixels) = value.strip_suffix("px").and_then(number) {
            Extent::Pixels(pixels)
        } else if let Some(percent) = value.strip_suffix('%').and_then(number) {
            Extent::Percent(percent)
        } else {
            number(value).map_or(Extent::Auto, Extent::Cells)
        }
    }

    /// The extent in pixels, given one cell and the whole terminal.
    fn resolve(self, cell: f32, whole: f32) -> Option<f32> {
        match self {
            Extent::Auto => None,
            Extent::Cells(cells) => Some(cells * cell),
            Extent::Pixels(pixels) => Some(pixels),
            Extent::Percent(percent) => Some(whole * percent / 100.),
        }
    }
}

pub struct ItermImage {
    pub image: RgbaImage,
    pub width: Extent,
    pub height: Extent,
    pub preserve_aspect: bool,
}

/// Decodes `args:base64`. Only images marked `inline=1` are shown; iTerm2
/// treats the rest as downloads.
pub fn decode_iterm(payload: &[u8]) -> Option<ItermImage> {
    let split = payload.iter().position(|&byte| byte == b':')?;
    let args = std::str::from_utf8(&payload[..split]).ok()?;
    let mut inline = false;
    let (mut width, mut height, mut preserve_aspect) = (Extent::Auto, Extent::Auto, true);
    for arg in args.split(';') {
        let Some((key, value)) = arg.split_once('=') else {
            continue;
        };
        match key {
            "inline" => inline = value == "1",
            "width" => width = Extent::parse(value),
            "height" => height = Extent::parse(value),
            "preserveAspectRatio" => preserve_aspect = value != "0",
            _ => {}
        }
    }
    if !inline {
        return None;
    }
    let encoded: Vec<u8> = payload[split + 1..]
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    if image.width() as usize > MAX_SIDE || image.height() as usize > MAX_SIDE {
        return None;
    }
    Some(ItermImage {
        image,
        width,
        height,
        preserve_aspect,
    })
}

impl ItermImage {
    /// Display size in pixels, inside a terminal of `whole` pixels whose cells
    /// are `cell` pixels.
    pub fn display_size(&self, cell: (f32, f32), whole: (f32, f32)) -> (f32, f32) {
        let (natural_w, natural_h) = (self.image.width() as f32, self.image.height() as f32);
        let width = self.width.resolve(cell.0, whole.0);
        let height = self.height.resolve(cell.1, whole.1);
        let (width, height) = match (width, height) {
            (None, None) => (natural_w, natural_h),
            (Some(w), None) => (w, natural_h * w / natural_w),
            (None, Some(h)) => (natural_w * h / natural_h, h),
            (Some(w), Some(h)) if self.preserve_aspect => {
                let scale = (w / natural_w).min(h / natural_h);
                (natural_w * scale, natural_h * scale)
            }
            (Some(w), Some(h)) => (w, h),
        };
        // Never wider than the terminal, as iTerm2 does.
        if width > whole.0 && whole.0 > 0. {
            (whole.0, height * whole.0 / width)
        } else {
            (width, height)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_all(parts: &[&[u8]]) -> Vec<Chunk> {
        let mut scanner = Scanner::default();
        let mut chunks: Vec<Chunk> = Vec::new();
        for part in parts {
            for chunk in scanner.scan(part) {
                // Merge adjacent text for easier comparison.
                match (chunks.last_mut(), chunk) {
                    (Some(Chunk::Text(last)), Chunk::Text(more)) => last.extend(more),
                    (_, chunk) => chunks.push(chunk),
                }
            }
        }
        chunks
    }

    #[test]
    fn scanner_splits_images_from_text_across_chunks() {
        let chunks = scan_all(&[b"a\x1bP0;1", b";0q#0~~\x1b", b"\\b"]);
        assert_eq!(
            chunks,
            [
                Chunk::Text(b"a".to_vec()),
                Chunk::Sixel(b"\x1bP0;1;0q#0~~".to_vec()),
                Chunk::Text(b"b".to_vec()),
            ]
        );
        let chunks = scan_all(&[b"\x1b]1337;Fi", b"le=inline=1:QQ==\x07z"]);
        assert_eq!(
            chunks,
            [
                Chunk::Iterm(b"inline=1:QQ==".to_vec()),
                Chunk::Text(b"z".to_vec())
            ]
        );
    }

    #[test]
    fn scanner_passes_other_sequences_through() {
        let other = b"\x1b[31mred\x1b]0;title\x07\x1bP$qm\x1b\\\x1b]8;;http://x\x1b\\\x1b\x1b[A";
        assert_eq!(scan_all(&[other]), [Chunk::Text(other.to_vec())]);
        // Split right after ESC.
        assert_eq!(
            scan_all(&[b"x\x1b", b"[1m"]),
            [Chunk::Text(b"x\x1b[1m".to_vec())]
        );
    }

    #[test]
    fn sixel_decodes_colours_repeats_and_bands() {
        // Two red pixels wide, 6 tall; then a band with one blue pixel on top.
        let image = decode_sixel(b"\x1bPq#1;2;100;0;0#1!2~-#2;2;0;0;100#2@").unwrap();
        assert_eq!((image.width(), image.height()), (2, 7));
        assert_eq!(image.get_pixel(1, 5).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(0, 6).0, [0, 0, 255, 255]);
        assert_eq!(image.get_pixel(1, 6).0, [0, 0, 0, 0]);
        // HLS: hue 120 is red in DEC's wheel.
        assert_eq!(hls(120, 50, 100), (255, 0, 0));
        assert!(decode_sixel(b"\x1bPq").is_none());
    }

    #[test]
    fn iterm_decodes_inline_images_and_sizes_them() {
        let mut png = Vec::new();
        RgbaImage::new(40, 20)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&png);
        let payload = format!("name=eA==;inline=1;width=10:{encoded}");
        let image = decode_iterm(payload.as_bytes()).unwrap();
        assert_eq!(image.width, Extent::Cells(10.));
        assert_eq!(image.display_size((8., 16.), (800., 600.)), (80., 40.));
        let download = format!("inline=0:{encoded}");
        assert!(decode_iterm(download.as_bytes()).is_none());
        assert_eq!(Extent::parse("50%"), Extent::Percent(50.));
        assert_eq!(Extent::parse("120px"), Extent::Pixels(120.));
        assert_eq!(Extent::parse("auto"), Extent::Auto);
    }
}
