//! Power-loss-tolerant records stored in two independent 4096-byte pages.
pub const PAYLOAD:usize=4038;
pub const PAGE:usize=4096;
pub fn crc(bytes:&[u8])->u32{let mut c=!0u32;for b in bytes{c^=*b as u32;for _ in 0..8{c=(c>>1)^(0xedb88320u32&0u32.wrapping_sub(c&1));}}!c}
pub fn newer(a:u32,b:u32)->bool{a!=b&&a.wrapping_sub(b)<0x8000_0000}

pub const LIGHTING: Format = Format::new(*b"PC04", PAYLOAD);

#[derive(Clone, Copy)]
pub struct Format {
    magic: [u8; 4],
    payload: usize,
}

impl Format {
    pub const fn new(magic: [u8; 4], payload: usize) -> Self {
        core::assert!(payload <= PAGE - 20);
        Self { magic, payload }
    }

    fn checksum(self, buf: &[u8; PAGE]) -> u32 {
        let mut c = !0u32;
        for b in buf[4..12].iter().chain(&buf[16..16 + self.payload]) {
            c ^= *b as u32;
            for _ in 0..8 {
                c = (c >> 1) ^ (0xedb88320u32 & 0u32.wrapping_sub(c & 1));
            }
        }
        !c
    }

    fn seal(self, buf: &mut [u8; PAGE], sequence: u32) {
        buf[..4].copy_from_slice(&self.magic);
        buf[4..8].copy_from_slice(&sequence.to_le_bytes());
        buf[8..12].copy_from_slice(&(self.payload as u32).to_le_bytes());
        let sum = self.checksum(buf);
        buf[12..16].copy_from_slice(&sum.to_le_bytes());
        buf[PAGE - 4..].fill(0);
    }

    pub fn valid(self, buf: &[u8; PAGE]) -> Option<u32> {
        if buf[..4] != self.magic
            || buf[8..12] != (self.payload as u32).to_le_bytes()
            || buf[PAGE - 4..] != [0; 4]
            || u32::from_le_bytes(buf[12..16].try_into().unwrap()) != self.checksum(buf)
        {
            return None;
        }
        Some(u32::from_le_bytes(buf[4..8].try_into().unwrap()))
    }

    pub fn latest<'a>(self, a: &'a [u8; PAGE], b: &'a [u8; PAGE]) -> Option<&'a [u8; PAGE]> {
        match (self.valid(a), self.valid(b)) {
            (Some(x), Some(y)) if newer(y, x) => Some(b),
            (Some(_), _) => Some(a),
            (None, Some(_)) => Some(b),
            _ => None,
        }
    }

    #[cfg(feature = "storage")]
    pub async fn commit<F: embedded_storage_async::nor_flash::NorFlash>(
        self, f: &mut F, base: u32, old: &mut [u8; PAGE], buf: &mut [u8; PAGE],
        snapshot: impl FnOnce(&mut [u8; PAGE]),
    ) -> bool {
        if f.read(base, old).await.is_err() || f.read(base + PAGE as u32, buf).await.is_err() {
            return false;
        }
        let (a, b) = (self.valid(old), self.valid(buf));
        let (target, sequence) = match (a, b) {
            (Some(x), Some(y)) if newer(y, x) => (base, y.wrapping_add(1)),
            (Some(x), _) => (base + PAGE as u32, x.wrapping_add(1)),
            (None, Some(y)) => (base, y.wrapping_add(1)),
            _ => (base, 1),
        };
        if target == base && b.is_some() {
            *old = *buf;
        }
        buf.fill(255);
        snapshot(buf);
        self.seal(buf, sequence);
        if (a.is_some() || b.is_some()) && old[16..16 + self.payload] == buf[16..16 + self.payload] {
            return true;
        }
        let body_end = (16 + self.payload).next_multiple_of(4);
        if f.erase(target, target + PAGE as u32).await.is_err()
            || f.write(target, &buf[..body_end]).await.is_err()
            || f.write(target + (PAGE - 4) as u32, &buf[PAGE - 4..]).await.is_err()
            || f.read(target, old).await.is_err()
        {
            return false;
        }
        self.valid(old) == Some(sequence) && old == buf
    }
}
