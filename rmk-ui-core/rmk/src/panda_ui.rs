//! PandaKB UI state and menu commands. No keymap or encoder assignments are overwritten.
use core::sync::atomic::{AtomicBool,AtomicU8,AtomicU16,AtomicU32,Ordering::Relaxed};
use crate::event::{KeyboardEvent,KeyboardEventPos};
use crate::input_device::rotary_encoder::Direction;
pub static LAYER:AtomicU8=AtomicU8::new(0);
pub static MODS:AtomicU8=AtomicU8::new(0);
pub static LEDS:AtomicU8=AtomicU8::new(0);
pub static WPM:AtomicU16=AtomicU16::new(0);
pub static RGB:AtomicU8=AtomicU8::new(3);
pub static BRIGHT:AtomicU8=AtomicU8::new(20);
pub static PAGE:AtomicU8=AtomicU8::new(0);
pub static SELECTION:AtomicU8=AtomicU8::new(0);
static MENU_BUTTONS:AtomicU8=AtomicU8::new(0);
static ESC_RELEASE:AtomicBool=AtomicBool::new(false);
static LAST_INPUT:AtomicU32=AtomicU32::new(0);
fn now()->u32{embassy_time::Instant::now().as_millis() as u32}
pub fn connection()->crate::types::connection::ConnectionStatus{crate::state::current_connection_status()}
pub fn battery()->crate::types::battery::BatteryStatus{crate::input_device::battery::current_battery_status()}
pub fn usb_powered()->bool{embassy_nrf::pac::POWER.usbregstatus().read().vbusdetect()}
pub fn touch(){LAST_INPUT.store(now(),Relaxed);}
pub fn timeout(){if PAGE.load(Relaxed)!=0&&now().wrapping_sub(LAST_INPUT.load(Relaxed))>30000 {PAGE.store(0,Relaxed);}}
fn open(page:u8){SELECTION.store(0,Relaxed);PAGE.store(page,Relaxed);touch();}
pub async fn command(id:u8){
 touch();match id {
  16=>{if PAGE.load(Relaxed)==0{open(1)}else{PAGE.store(0,Relaxed)}},
  17=>rotate(false),18=>rotate(true),19=>select().await,20=>{match PAGE.load(Relaxed){3=>open(2),2=>open(1),_=>PAGE.store(0,Relaxed)}},
  21=>{RGB.store((RGB.load(Relaxed)+1)%5,Relaxed);},
  22=>crate::panda_lighting::adjust_brightness(true),
  23=>crate::panda_lighting::adjust_brightness(false),
  24=>{RGB.store(if RGB.load(Relaxed)==0{3}else{0},Relaxed);},
  25=>open(2),_=>{}
 }
}
pub fn rotate(next:bool){
 touch();let page=PAGE.load(Relaxed);
 if page==0{return}
 if page==3 {crate::panda_lighting::adjust_brightness(next);return;}
 let count=if page==1{8}else{7};let old=SELECTION.load(Relaxed);
 SELECTION.store(if next{(old+1)%count}else{(old+count-1)%count},Relaxed);
}
async fn prefer(transport:crate::types::connection::ConnectionType){
 crate::state::set_preferred_connection(transport);
 crate::channel::FLASH_CHANNEL.send(crate::storage::FlashOperationMessage::ConnectionType(transport)).await;
}
pub async fn select(){
 touch();let choice=SELECTION.load(Relaxed);
 match PAGE.load(Relaxed){
  1=>match choice {
   0..=3=>{prefer(crate::types::connection::ConnectionType::Ble).await;let profile=if choice==0{crate::ble::profile::DONGLE_PROFILE}else{choice-1};crate::channel::BLE_PROFILE_CHANNEL.send(crate::ble::profile::BleProfileAction::Switch(profile)).await;PAGE.store(0,Relaxed);},
   4=>{prefer(crate::types::connection::ConnectionType::Usb).await;PAGE.store(0,Relaxed);},
   5=>open(2),
   6=>{crate::channel::BLE_PROFILE_CHANNEL.send(crate::ble::profile::BleProfileAction::ClearBond).await;PAGE.store(0,Relaxed);},
   _=>{PAGE.store(0,Relaxed);}
  },
  2=>{if choice<5{RGB.store(choice,Relaxed);PAGE.store(0,Relaxed);}else if choice==5{PAGE.store(3,Relaxed);}else{open(1);}},
  3=>open(2),_=>open(1)
 }
}
/// 0 = normal event, 1 = consumed by the already-open menu.
pub async fn intercept(event:KeyboardEvent)->u8{
 let menu=PAGE.load(Relaxed)!=0;
 match event.pos{
  KeyboardEventPos::RotaryEncoder(e) if menu=>{if event.pressed&&e.direction!=Direction::None{rotate(e.direction==Direction::Clockwise);}return 1;},
  KeyboardEventPos::Key(k)=>{
   if k.row==0&&k.col==0{
    if menu&&event.pressed{PAGE.store(0,Relaxed);ESC_RELEASE.store(true,Relaxed);return 1;}
    if !event.pressed&&ESC_RELEASE.swap(false,Relaxed){return 1;}
   }
   if (k.row==4||k.row==9)&&k.col==5{
    let mask=if k.row==4{1}else{2};
    if event.pressed&&menu{MENU_BUTTONS.fetch_or(mask,Relaxed);return 1;}
    if !event.pressed&&MENU_BUTTONS.fetch_and(!mask,Relaxed)&mask!=0{if menu{select().await;}return 1;}
   }
  },_=>{}
 }0
}
