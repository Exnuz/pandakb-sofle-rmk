//! Separate flash records for lighting and Vial overrides on LEFT.
use core::cell::Cell;
use core::sync::atomic::{AtomicBool,AtomicU32,Ordering::Relaxed};
use embassy_sync::{mutex::Mutex,signal::Signal};
use embedded_storage_async::nor_flash::{NorFlash,ReadNorFlash};
use crate::{
    panda_colors as colors, panda_lighting as light, panda_ui as ui, panda_record as record,
    panda_override::{COUNT, PAYLOAD_BYTES, RULE_BYTES, Rule},
};
const BASE:u32=0xE8000;
const OVERRIDE_BASE: u32 = 0xEA000;
const OVERRIDE_FORMAT: record::Format = record::Format::new(*b"PKO1", PAYLOAD_BYTES);
type Flash=nrf_mpsl::Flash<'static>;
type FlashError=<Flash as embedded_storage::nor_flash::ErrorType>::Error;
// MPSL Flash is intentionally !Send. Access only from the single thread-mode executor.
static FLASH:embassy_sync::blocking_mutex::ThreadModeMutex<Mutex<embassy_sync::blocking_mutex::raw::NoopRawMutex,Option<Flash>>>=embassy_sync::blocking_mutex::ThreadModeMutex::new(Mutex::new(None));
static READY:AtomicBool=AtomicBool::new(false);
static GATE:Mutex<crate::RawMutex,()>=Mutex::new(());
static NEXT:AtomicU32=AtomicU32::new(1);
pub(crate) static SAVE:Signal<crate::RawMutex,SaveRequest>=Signal::new();
static DONE:Signal<crate::RawMutex,(u32,bool)>=Signal::new();
static OVERRIDES: embassy_sync::blocking_mutex::Mutex<crate::RawMutex, Cell<[Rule; COUNT]>> =
    embassy_sync::blocking_mutex::Mutex::new(Cell::new([Rule::decode([0; RULE_BYTES]); COUNT]));

enum SaveKind {
    Lighting,
    Override { index: usize, rule: Rule },
}

pub(crate) struct SaveRequest {
    id: u32,
    kind: SaveKind,
}

#[repr(align(4))]struct Page([u8;4096]);
#[derive(Clone,Copy)]pub struct SharedFlash;
impl embedded_storage::nor_flash::ErrorType for SharedFlash{type Error=FlashError;}
impl ReadNorFlash for SharedFlash{
 const READ_SIZE:usize=1;
 async fn read(&mut self,offset:u32,bytes:&mut[u8])->Result<(),Self::Error>{let mut g=FLASH.borrow().lock().await;ReadNorFlash::read(g.as_mut().unwrap(),offset,bytes).await}
 fn capacity(&self)->usize{1024*1024}
}
impl NorFlash for SharedFlash{
 const WRITE_SIZE:usize=4;const ERASE_SIZE:usize=4096;
 async fn write(&mut self,offset:u32,bytes:&[u8])->Result<(),Self::Error>{let mut g=FLASH.borrow().lock().await;NorFlash::write(g.as_mut().unwrap(),offset,bytes).await}
 async fn erase(&mut self,from:u32,to:u32)->Result<(),Self::Error>{let mut g=FLASH.borrow().lock().await;NorFlash::erase(g.as_mut().unwrap(),from,to).await}
}
fn snapshot(buf:&mut[u8;4096]){
 let mut n=16;for data in [&colors::BASE[..],&colors::BLINK[..],&colors::FLAGS[..]]{for v in data{buf[n]=v.load(Relaxed);n+=1;}}
 // Final byte reserved: ignore the removed legacy encoder-hold setting on restore.
 for v in [ui::RGB.load(Relaxed),ui::BRIGHT.load(Relaxed),light::HUE.load(Relaxed),light::SAT.load(Relaxed),light::SPEED.load(Relaxed),0]{buf[n]=v;n+=1;}
}
fn restore(buf:&[u8;4096]){
 let mut n=16;for data in [&colors::BASE[..],&colors::BLINK[..],&colors::FLAGS[..]]{for v in data{v.store(buf[n],Relaxed);n+=1;}}
 ui::RGB.store(buf[n].min(8),Relaxed);ui::BRIGHT.store(buf[n+1].min(100),Relaxed);light::HUE.store(buf[n+2],Relaxed);light::SAT.store(buf[n+3],Relaxed);light::SPEED.store(buf[n+4],Relaxed);
}
pub async fn share(flash:Flash)->SharedFlash{
 *FLASH.borrow().lock().await=Some(flash);
 let mut a=Page([255;4096]);let mut b=Page([255;4096]);let mut f=SharedFlash;
 let _=f.read(BASE,&mut a.0).await;let _=f.read(BASE+4096,&mut b.0).await;
 if let Some(page) = record::LIGHTING.latest(&a.0, &b.0) {
  restore(page);
 }
 let _ = f.read(OVERRIDE_BASE, &mut a.0).await;
 let _ = f.read(OVERRIDE_BASE + record::PAGE as u32, &mut b.0).await;
 if let Some(page) = OVERRIDE_FORMAT.latest(&a.0, &b.0) {
  OVERRIDES.lock(|table| table.set(core::array::from_fn(|index| {
   let start = 16 + index * RULE_BYTES;
   let rule = Rule::decode(page[start..start + RULE_BYTES].try_into().unwrap());
   if rule.supported() { rule } else { Rule::default() }
  })));
 }
 READY.store(true,Relaxed);f
}
pub async fn save()->bool{
 request(SaveKind::Lighting).await
}

pub fn with_overrides<T>(f: impl FnOnce(&[Rule; COUNT]) -> T) -> T {
    OVERRIDES.lock(|table| f(&table.get()))
}

pub async fn save_override(index: usize, rule: Rule) -> bool {
    if index >= COUNT || !rule.supported() {
        return false;
    }
    request(SaveKind::Override { index, rule }).await
}

async fn request(kind: SaveKind)->bool{
 if !READY.load(Relaxed){return false;}
 let _guard=GATE.lock().await;let id=NEXT.fetch_add(1,Relaxed);
 DONE.reset();SAVE.signal(SaveRequest { id, kind });
 loop{let (finished,result)=DONE.wait().await;if finished==id{return result;}}
}

pub(crate) async fn commit(request: SaveRequest) {
 let result = match request.kind {
  SaveKind::Lighting => write_lighting().await,
  SaveKind::Override { index, rule } => {
   let mut rules = with_overrides(|table| *table);
   rules[index] = rule;
   write_overrides(rules).await
  }
 };
 DONE.signal((request.id, result));
}

async fn write_lighting()->bool{
 let mut old=Page([255;4096]);let mut buf=Page([255;4096]);
 record::LIGHTING.commit(&mut SharedFlash,BASE,&mut old.0,&mut buf.0,snapshot).await
}

pub(crate) async fn clear_overrides() -> bool {
    !READY.load(Relaxed) || write_overrides([Rule::default(); COUNT]).await
}

async fn write_overrides(rules: [Rule; COUNT]) -> bool {
    if with_overrides(|current| *current == rules) {
        return true;
    }
    let mut old = Page([255; record::PAGE]);
    let mut buf = Page([255; record::PAGE]);
    let saved = OVERRIDE_FORMAT.commit(&mut SharedFlash, OVERRIDE_BASE, &mut old.0, &mut buf.0, |page| {
        for (data, rule) in page[16..16 + PAYLOAD_BYTES].chunks_exact_mut(RULE_BYTES).zip(rules) {
            data.copy_from_slice(&rule.encode());
        }
    }).await;
    if saved {
        OVERRIDES.lock(|table| table.set(rules));
    }
    saved
}
