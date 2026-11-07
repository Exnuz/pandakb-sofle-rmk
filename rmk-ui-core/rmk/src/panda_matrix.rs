use crate::event::KeyboardEvent;
/// Recording the matrix pressed state
pub struct MatrixState {
    // 30 bytes is the limit by Vial and 240 keys is enough for most keyboards
    state: [u8; 30],
    row: usize,
    col: usize,
    row_len: usize,
}

impl MatrixState {
    pub fn new(row: usize, col: usize) -> Self {
        let row_len = col.div_ceil(8);
        assert!(row * row_len <= 30, "Matrix too large for MatrixState");
        Self {
            state: [0; 30],
            row,
            col,
            row_len,
        }
    }
    pub fn update(&mut self, event: &KeyboardEvent) {
        use crate::event::{KeyPos, KeyboardEventPos};
        if let KeyboardEventPos::Key(KeyPos { row, col }) = event.pos {
            if row as usize >= self.row || col as usize >= self.col {
                warn!("Matrix read out of bounds");
                return;
            }
            let pressed = event.pressed;
            let index = row as usize * self.row_len * 8 + col as usize;
            let byte_index = index / 8;
            let bit_index = index % 8;
            self.state[byte_index] = self.state[byte_index] & !(1 << bit_index) | ((pressed as u8) << bit_index);
        }
    }
    /// Copy the bitmap out in natural row-major order (bit 0 = col 0,
    /// low byte first within a row) — the Rynk `MatrixState` wire order.
    pub fn read_all(&self, target: &mut [u8]) {
        let n = (self.row * self.row_len).min(target.len());
        target[..n].copy_from_slice(&self.state[..n]);
    }
    pub fn read(&self, row: u8, col: u8) -> bool {
        if row as usize >= self.row || col as usize >= self.col {
            warn!("Matrix read out of bounds");
            return false;
        }
        let index = row as usize * self.row_len * 8 + col as usize;
        let byte_index = index / 8;
        let bit_index = index % 8;
        self.state[byte_index] & (1 << bit_index) != 0
    }
}

