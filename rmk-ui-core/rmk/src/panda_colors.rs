//! Compatible with the preserved PandaKB key-colors WebHID editor (8 layers, 72 LEDs).
use core::sync::atomic::{AtomicU8,Ordering::Relaxed};
pub static BASE:[AtomicU8;1728]=[const{AtomicU8::new(255)};1728];
pub static BLINK:[AtomicU8;1728]=[const{AtomicU8::new(0)};1728];
pub static FLAGS:[AtomicU8;576]=[const{AtomicU8::new(0)};576];
pub static REMOTE:[AtomicU8;108]=[const{AtomicU8::new(0)};108];

pub const PALETTE_PART_BYTES: usize = 27;
pub const PALETTE_BYTES: usize = 108;
pub const PALETTE_PARTS: u8 = (PALETTE_BYTES / PALETTE_PART_BYTES) as u8;

/// Apply a palette only once all four consecutive chunks have arrived.
#[derive(Default)]
pub struct RemotePalette {
    next_part: u8,
    bytes: [[u8; PALETTE_PART_BYTES]; PALETTE_PARTS as usize],
}

impl RemotePalette {
    pub fn receive(&mut self, offset: u8, data: [u8; PALETTE_PART_BYTES]) {
        const LEDS_PER_PART: u8 = (PALETTE_PART_BYTES / 3) as u8;
        if offset == 0 {
            self.next_part = 0;
        }
        if offset != self.next_part * LEDS_PER_PART {
            self.next_part = 0;
            return;
        }
        self.bytes[self.next_part as usize] = data;
        self.next_part += 1;
        if self.next_part == PALETTE_PARTS {
            for (target, value) in REMOTE.iter().zip(self.bytes.iter().flatten()) {
                target.store(*value, Relaxed);
            }
            self.next_part = 0;
        }
    }
}
pub fn packet(req:&[u8;32],reply:&mut[u8;32])->bool{
 if req[0]!=2||!(0xa4..=0xab).contains(&req[1]){return false;}
 *reply=*req;
 if req[1]==0xa4{crate::panda_ui::RGB.store(8,Relaxed);return true;}
 if req[1]==0xa6{reply[0]=255;return true;} // No false acknowledgement of nonvolatile storage.
 let (layer,offset,count)=(req[2]as usize,req[3]as usize,req[4]as usize);
 let flags=matches!(req[1],0xa8|0xa9);let max=if flags{27}else{9};
 if layer>=8||offset>=72||count==0||count>max||offset+count>72{reply[0]=255;return true;}
 let get=matches!(req[1],0xa7|0xa9|0xab);let width=if flags{1}else{3};
 let data=if flags{&FLAGS[..]}else if matches!(req[1],0xaa|0xab){&BLINK[..]}else{&BASE[..]};
 for i in 0..count*width{let index=(layer*72+offset)*width+i;if get{reply[5+i]=data[index].load(Relaxed);}else{data[index].store(req[5+i],Relaxed);}}
 true
}
pub fn color(layer:u8,led:usize,tick:u64)->[u8;3]{
 let key=layer.min(7)as usize*72+led;let flags=FLAGS[key].load(Relaxed);let leds=crate::panda_ui::LEDS.load(Relaxed);
 if flags&7!=0&&flags&leds&7==0{return[0;3];}
 let data=if flags&8!=0&&(tick/800)%2==1{&BLINK}else{&BASE};
 core::array::from_fn(|i|data[key*3+i].load(Relaxed))
}
