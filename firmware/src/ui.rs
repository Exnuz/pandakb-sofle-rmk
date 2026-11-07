// SPDX-License-Identifier: GPL-2.0-or-later
//! Physical OLED/RGB adapter for the approved PandaKB interface.
#[path="assets.rs"]mod assets;
#[path="pixel_font.rs"]mod pixel_font;
#[path="render.rs"]mod render;
#[path="display_update.rs"]mod display_update;
use core::sync::atomic::{AtomicU8,Ordering::Relaxed};
use embassy_nrf::{bind_interrupts,gpio::{Output,Level,OutputDrive},peripherals::*,Peri,spim,twim};
use embassy_time::{Duration,Instant,Timer};
use rmk::panda_ui as state;
use rmk::types::{self, battery::BatteryStatus};
pub static PHASE:AtomicU8=AtomicU8::new(1);
bind_interrupts!(struct UiIrqs{TWISPI0=>twim::InterruptHandler<TWISPI0>;SPIM3=>spim::InterruptHandler<SPI3>;});
async fn command(i2c:&mut twim::Twim<'static>,bytes:&[u8])->bool{let mut b=[0u8;32];b[1..bytes.len()+1].copy_from_slice(bytes);embassy_time::with_timeout(Duration::from_millis(15),i2c.write(0x3c,&b[..bytes.len()+1])).await.is_ok_and(|r|r.is_ok())}
async fn display(i2c:&mut twim::Twim<'static>,frame:&[u8;512],previous:&[u8;512],initialized:&mut bool)->bool{
 let force=!*initialized;
 if !*initialized{
  if !command(i2c,&[0xAE,0xD5,0x80,0xA8,31,0xD3,0,0x40,0x8D,0x14,0x20,0,0xA1,0xC8,0xDA,2,0x81,0x5F,0xD9,0xF1,0xDB,0x40,0xA4,0xA6,0xAF]).await{return false;}*initialized=true;
 }
 for page in 0..4{
  let base=page*128;
  let Some((start,end))=display_update::dirty_span(&previous[base..base+128],&frame[base..base+128],force)else{continue;};
  if !command(i2c,&[0x21,start as u8,(end-1)as u8,0x22,page as u8,page as u8]).await{*initialized=false;return false;}
  for bytes in frame[base+start..base+end].chunks(16){let mut b=[0x40;17];b[1..1+bytes.len()].copy_from_slice(bytes);if !embassy_time::with_timeout(Duration::from_millis(15),i2c.write(0x3c,&b[..bytes.len()+1])).await.is_ok_and(|r|r.is_ok()){*initialized=false;return false;}}
 }
 true
}
fn led_frame(mode: u8, bright: u8, layer: u8, tick: u64) -> [[u8; 3]; 36] {
 core::array::from_fn(|i| {
  if mode == 8 && env!("CARGO_BIN_NAME") == "right" {
   core::array::from_fn(|c| (rmk::panda_colors::REMOTE[i*3+c].load(Relaxed) as u32 * bright.min(100) as u32 * 48 / (255*100)) as u8)
  } else {
   rmk::panda_lighting::color(mode, bright, layer, tick, i)
  }
 })
}
async fn write_leds(spi: &mut spim::Spim<'static>, frame: &[[u8; 3]; 36]) -> bool {
 let mut b=[0u8;36*24+64];
 for (i, rgb) in frame.iter().enumerate(){
  let grb=[rgb[1],rgb[0],rgb[2]];
  for c in 0..3{for bit in 0..8{b[i*24+c*8+bit]=if grb[c]&(0x80>>bit)!=0{0x70}else{0x40};}}
 }
 spi.write(&b).await.is_ok()
}
pub async fn start(spawner:embassy_executor::Spawner,bus:Peri<'static,TWISPI0>,spi:Peri<'static,SPI3>,sda:Peri<'static,P0_17>,scl:Peri<'static,P0_20>,mosi:Peri<'static,P0_06>,power:Peri<'static,P0_13>){
 let power=Output::new(power,Level::High,OutputDrive::Standard);Timer::after_millis(50).await;
 let mut cfg=twim::Config::default();cfg.frequency=twim::Frequency::K400;
 let i2c=twim::Twim::new(bus,UiIrqs,sda,scl,cfg,&mut []);
 let mut cfg=spim::Config::default();cfg.frequency=spim::Frequency::M4;
 let spi=spim::Spim::new_txonly_nosck(spi,UiIrqs,mosi,cfg);
 spawner.spawn(ui_task(i2c,power).unwrap());
 spawner.spawn(rgb_task(spi).unwrap());
 Timer::after_millis(400).await;
}
#[embassy_executor::task]
async fn rgb_task(mut spi: spim::Spim<'static>) {
 let mut ticker = embassy_time::Ticker::every(Duration::from_millis(25));
 let mut previous = None;
 loop {
  let frame = led_frame(state::RGB.load(Relaxed), state::BRIGHT.load(Relaxed), state::LAYER.load(Relaxed), Instant::now().as_millis());
  if previous.as_ref() != Some(&frame) && write_leds(&mut spi, &frame).await {
   previous = Some(frame);
  }
  ticker.next().await;
 }
}
#[embassy_executor::task]
async fn ui_task(mut i2c:twim::Twim<'static>,_power:Output<'static>){
 let left=env!("CARGO_BIN_NAME")=="left";
 let mut initialized=false;let mut previous=[0xFFu8;512];let mut history=display_update::History::new();let mut smooth=display_update::SmoothWpm::new();
 let mut ui_tick=embassy_time::Ticker::every(Duration::from_millis(50));
 loop{
  let tick=Instant::now().as_millis();if left{state::timeout();}
  let (mode, link) = render::connection_badge(state::connection(), left || rmk::DIAGNOSTIC_SPLIT.load(Relaxed));
  let wpm=if left||rmk::DIAGNOSTIC_SPLIT.load(Relaxed){state::WPM.load(Relaxed)}else{0};
  history.update(tick,wpm);
  let shown=if left{wpm}else{smooth.update(wpm,tick)};
  let live_history=history.live(shown);
  let battery=match state::battery(){BatteryStatus::Available{level,..}=>level,_=>None};
  let model=render::Model{wpm:shown,layer:state::LAYER.load(Relaxed),mods:state::MODS.load(Relaxed),leds:state::LEDS.load(Relaxed),battery,usb:state::usb_powered(),mode,link,page:if left{state::PAGE.load(Relaxed)}else{0},selection:state::SELECTION.load(Relaxed),brightness:state::BRIGHT.load(Relaxed),history:&live_history};
  let frame=if tick<2500 {let mut f=render::Frame::new();f.center("UI RC8",36,1);f.center(if left{"LEFT"}else{"RIGHT"},52,1);f.hardware()}else{render::Frame::draw(left,&model,tick).hardware()};
  if frame!=previous||!initialized{if display(&mut i2c,&frame,&previous,&mut initialized).await{previous=frame;}}
  ui_tick.next().await;
 }
}
