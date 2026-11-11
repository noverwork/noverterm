//! Lays the terminal grid out as background quads and shaped text runs.

use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};
use gpui_kit::*;

use super::{GridSize, TerminalEvent, TerminalView, PADDING};

const ANSI: [u32; 16] = [
    0x1f2937, 0xef4444, 0x22c55e, 0xeab308, 0x3b82f6, 0xa855f7, 0x06b6d4, 0xe5e5e5, 0x404040,
    0xf87171, 0x4ade80, 0xfacc15, 0x60a5fa, 0xc084fc, 0x22d3ee, 0xffffff,
];
const FOREGROUND: u32 = 0xe5e5e5;
const BACKGROUND: u32 = 0x000000;
const SELECTION: u32 = 0x2563eb;
const SEARCH_MATCH: u32 = 0xe6c27a;

fn rgb_of(value: u32) -> Rgb {
    Rgb {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
    }
}

/// The colour at `index` in alacritty's table when the program did not set one:
/// 0-15 ANSI, 16-231 the 6x6x6 cube, 232-255 greys, then the named extras.
pub(super) fn default_color(index: usize) -> Rgb {
    let dim = |value: u32| {
        let color = rgb_of(value);
        Rgb {
            r: (color.r as f32 * 0.66) as u8,
            g: (color.g as f32 * 0.66) as u8,
            b: (color.b as f32 * 0.66) as u8,
        }
    };
    match index {
        0..=15 => rgb_of(ANSI[index]),
        16..=231 => {
            let level = |step: usize| if step == 0 { 0 } else { (55 + step * 40) as u8 };
            let index = index - 16;
            Rgb {
                r: level(index / 36),
                g: level(index / 6 % 6),
                b: level(index % 6),
            }
        }
        232..=255 => {
            let grey = (8 + (index - 232) * 10) as u8;
            Rgb {
                r: grey,
                g: grey,
                b: grey,
            }
        }
        index if index == NamedColor::Background as usize => rgb_of(BACKGROUND),
        index
            if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize).contains(&index) =>
        {
            dim(ANSI[index - NamedColor::DimBlack as usize])
        }
        index if index == NamedColor::DimForeground as usize => dim(FOREGROUND),
        _ => rgb_of(FOREGROUND),
    }
}

fn resolve(color: Color, colors: &Colors) -> Rgb {
    match color {
        Color::Spec(rgb) => rgb,
        Color::Indexed(index) => {
            colors[index as usize].unwrap_or_else(|| default_color(index as usize))
        }
        Color::Named(name) => colors[name].unwrap_or_else(|| default_color(name as usize)),
    }
}

fn hsla(color: Rgb) -> Hsla {
    Rgba {
        r: color.r as f32 / 255.,
        g: color.g as f32 / 255.,
        b: color.b as f32 / 255.,
        a: 1.,
    }
    .into()
}

#[derive(Clone, PartialEq)]
struct RunStyle {
    foreground: Hsla,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

/// One frame of terminal content, ready to paint.
pub(super) struct Frame {
    backgrounds: Vec<(Bounds<Pixels>, Hsla)>,
    runs: Vec<(Point<Pixels>, ShapedLine)>,
    cursor: Option<PaintQuad>,
    line_height: Pixels,
}

impl Frame {
    pub(super) fn paint(self, window: &mut Window, cx: &mut App) {
        for (bounds, color) in self.backgrounds {
            window.paint_quad(fill(bounds, color));
        }
        for (origin, line) in self.runs {
            let _ = line.paint(origin, self.line_height, TextAlign::Left, None, window, cx);
        }
        if let Some(cursor) = self.cursor {
            window.paint_quad(cursor);
        }
    }
}

impl TerminalView {
    pub(super) fn cursor_bounds(&self) -> Bounds<Pixels> {
        let cursor = self.term.grid().cursor.point;
        let row = cursor.line.0 + self.term.grid().display_offset() as i32;
        Bounds::new(
            point(
                self.bounds.origin.x + PADDING + self.cell.width * cursor.column.0 as f32,
                self.bounds.origin.y + self.cell.height * row as f32,
            ),
            self.cell,
        )
    }

    pub(super) fn layout(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Frame {
        let font = self.font();
        let font_size = px(self.config.font_size);
        let text_system = window.text_system().clone();
        let font_id = text_system.resolve_font(&font);
        let cell_width = text_system
            .advance(font_id, font_size, 'm')
            .map(|advance| advance.width)
            .unwrap_or(font_size * 0.6);
        let line_height = (text_system.ascent(font_id, font_size)
            + text_system.descent(font_id, font_size).abs())
        .ceil();
        self.cell = size(cell_width, line_height);
        self.bounds = bounds;

        let cols = (f32::from(bounds.size.width - PADDING * 2.) / f32::from(cell_width)) as usize;
        let rows = (f32::from(bounds.size.height) / f32::from(line_height)) as usize;
        let fitted = GridSize {
            cols: cols.max(2),
            rows: rows.max(1),
        };
        if fitted != self.grid {
            self.set_grid(fitted.cols, fitted.rows, cx);
            cx.emit(TerminalEvent::Resized {
                cols: fitted.cols,
                rows: fitted.rows,
            });
        }

        let focused = self.focus.is_focused(window);
        let content = self.term.renderable_content();
        let colors = content.colors;
        let display_offset = content.display_offset as i32;
        let cursor_point = content.cursor.point;
        let blinking = self.term.cursor_style().blinking;
        let cursor_shape = match content.cursor.shape {
            CursorShape::Hidden => None,
            _ if display_offset != 0 => None,
            _ if focused && blinking && !self.blink_on => None,
            shape if focused => Some(shape),
            _ => Some(CursorShape::HollowBlock),
        };
        let cursor_color = hsla(resolve(Color::Named(NamedColor::Cursor), colors));
        let default_background = resolve(Color::Named(NamedColor::Background), colors);
        let origin = point(bounds.origin.x + PADDING, bounds.origin.y);
        let cell_origin = |row: i32, column: usize| {
            point(
                origin.x + cell_width * column as f32,
                origin.y + line_height * row as f32,
            )
        };

        let mut backgrounds: Vec<(Bounds<Pixels>, Hsla)> = Vec::new();
        let mut runs = Vec::new();
        // The run being collected: its row, first column, next column, text.
        let mut pending: Option<(i32, usize, usize, String, RunStyle)> = None;
        let mut flush = |pending: &mut Option<(i32, usize, usize, String, RunStyle)>| {
            let Some((row, column, _, text, style)) = pending.take() else {
                return;
            };
            let run = TextRun {
                len: text.len(),
                font: Font {
                    weight: if style.bold {
                        FontWeight::BOLD
                    } else {
                        font.weight
                    },
                    style: if style.italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    },
                    ..font.clone()
                },
                color: style.foreground,
                background_color: None,
                underline: style.underline.then_some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(style.foreground),
                    wavy: false,
                }),
                strikethrough: style.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(style.foreground),
                }),
            };
            let line = text_system.shape_line(text.into(), font_size, &[run], Some(cell_width));
            runs.push((cell_origin(row, column), line));
        };

        for cell in content.display_iter {
            let row = cell.point.line.0 + display_offset;
            let column = cell.point.column.0;
            let flags = cell.flags;
            if flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                continue;
            }
            let width = if flags.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };

            let mut foreground = resolve(cell.fg, colors);
            let mut background = resolve(cell.bg, colors);
            if flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }
            let selected = content
                .selection
                .is_some_and(|selection| selection.contains(cell.point));
            let matched = self
                .search_match
                .as_ref()
                .is_some_and(|found| found.contains(&cell.point));
            let block_cursor =
                cell.point == cursor_point && matches!(cursor_shape, Some(CursorShape::Block));
            let mut foreground = hsla(foreground);
            let mut fill_color = (background != default_background).then(|| hsla(background));
            if matched {
                fill_color = Some(rgb(SEARCH_MATCH).into());
                foreground = rgb(BACKGROUND).into();
            } else if selected {
                fill_color = Some(rgb(SELECTION).into());
            }
            if block_cursor {
                fill_color = Some(cursor_color);
                foreground = hsla(default_background);
            }
            if flags.contains(Flags::DIM) {
                foreground.a *= 0.66;
            }

            if let Some(color) = fill_color {
                let cell_bounds = Bounds::new(
                    cell_origin(row, column),
                    size(cell_width * width as f32, line_height),
                );
                match backgrounds.last_mut() {
                    // Extend the previous quad instead of painting one per cell.
                    Some((last, last_color))
                        if *last_color == color
                            && last.origin.y == cell_bounds.origin.y
                            && last.right() == cell_bounds.origin.x =>
                    {
                        last.size.width += cell_bounds.size.width;
                    }
                    _ => backgrounds.push((cell_bounds, color)),
                }
            }

            let decorated = flags.intersects(Flags::ALL_UNDERLINES | Flags::STRIKEOUT);
            if flags.contains(Flags::HIDDEN) || (cell.c == ' ' && !decorated) {
                flush(&mut pending);
                continue;
            }
            let style = RunStyle {
                foreground,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.intersects(Flags::ALL_UNDERLINES),
                strikethrough: flags.contains(Flags::STRIKEOUT),
            };
            let continues = pending
                .as_ref()
                .is_some_and(|(run_row, _, next, _, run_style)| {
                    *run_row == row && *next == column && *run_style == style
                });
            if !continues {
                flush(&mut pending);
                pending = Some((row, column, column, String::new(), style));
            }
            if let Some((_, _, next, text, _)) = pending.as_mut() {
                text.push(cell.c);
                text.extend(cell.zerowidth().into_iter().flatten());
                *next = column + width;
            }
            // A wide glyph takes two cells but one forced advance: start over
            // so the next cell lands on its own column.
            if width == 2 {
                flush(&mut pending);
            }
        }
        flush(&mut pending);

        let cursor_cell = Bounds::new(
            cell_origin(cursor_point.line.0 + display_offset, cursor_point.column.0),
            size(cell_width, line_height),
        );
        let cursor = match cursor_shape {
            Some(CursorShape::Beam) => Some(fill(
                Bounds::new(cursor_cell.origin, size(px(2.), line_height)),
                cursor_color,
            )),
            Some(CursorShape::Underline) => Some(fill(
                Bounds::new(
                    point(cursor_cell.origin.x, cursor_cell.bottom() - px(2.)),
                    size(cell_width, px(2.)),
                ),
                cursor_color,
            )),
            Some(CursorShape::HollowBlock) => {
                Some(outline(cursor_cell, cursor_color, BorderStyle::Solid))
            }
            _ => None,
        };

        // Text the input method is composing sits on top of the cursor.
        if let Some(marked) = self.marked_text.clone() {
            let run = TextRun {
                len: marked.len(),
                font: font.clone(),
                color: rgb(FOREGROUND).into(),
                background_color: None,
                underline: Some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(rgb(FOREGROUND).into()),
                    wavy: false,
                }),
                strikethrough: None,
            };
            let line = text_system.shape_line(marked.into(), font_size, &[run], None);
            backgrounds.push((
                Bounds::new(cursor_cell.origin, size(line.width, line_height)),
                rgb(BACKGROUND).into(),
            ));
            runs.push((cursor_cell.origin, line));
        }

        Frame {
            backgrounds,
            runs,
            cursor,
            line_height,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the GPUI glob would shadow `#[test]` with its own macro.
    use super::{default_color, rgb_of, BACKGROUND, FOREGROUND};
    use alacritty_terminal::vte::ansi::{NamedColor, Rgb};

    #[test]
    fn default_palette_covers_cube_greys_and_named() {
        assert_eq!(default_color(1), rgb_of(0xef4444));
        assert_eq!(default_color(16), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            default_color(231),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(default_color(232), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(
            default_color(NamedColor::Background as usize),
            rgb_of(BACKGROUND)
        );
        assert_eq!(
            default_color(NamedColor::Foreground as usize),
            rgb_of(FOREGROUND)
        );
    }
}
