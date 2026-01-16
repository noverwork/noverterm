// VT100/VT220 Terminal Renderer
// Converts terminal output to renderable data for Canvas

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Cell {
    pub ch: char,
    pub fg: u32,  // RGB color
    pub bg: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: 0x00F1_F5F9, // slate-100
            bg: 0x000F_172A, // slate-900
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TerminalGrid {
    pub rows: u16,
    pub cols: u16,
    pub grid: Vec<Vec<Cell>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

impl TerminalGrid {
    pub fn new(rows: u16, cols: u16) -> Self {
        let grid = vec![vec![Cell::default(); cols as usize]; rows as usize];
        TerminalGrid {
            rows,
            cols,
            grid,
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        self.grid = vec![vec![Cell::default(); cols as usize]; rows as usize];
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    // Simple VT100 sequence parser
    pub fn write(&mut self, data: &[u8]) {
        for &byte in data {
            match byte {
                0x08 => self.backspace(),
                0x09 => self.tab(),
                0x0A => self.linefeed(),
                0x0D => self.carriage_return(),
                0x1B => self.escape_sequence(), // TODO: parse CSI
                c if c.is_ascii_graphic() => self.print_char(c as char),
                _ => {}
            }
        }
    }

    fn print_char(&mut self, ch: char) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;

        if row < self.grid.len() && col < self.grid[row].len() {
            self.grid[row][col].ch = ch;
            self.cursor_col += 1;
        }
    }

    fn linefeed(&mut self) {
        if self.cursor_row < self.rows - 1 {
            self.cursor_row += 1;
        }
        self.cursor_col = 0;
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn tab(&mut self) {
        self.cursor_col = (self.cursor_col / 8 + 1) * 8;
        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols - 1;
        }
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn escape_sequence(&mut self) {
        // TODO: Implement full VT100 CSI sequences
    }
}
