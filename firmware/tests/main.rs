//! Runs the actual firmware menu and framebuffer code with mocked hardware/transport.
#![allow(dead_code,non_upper_case_globals)]
macro_rules! warn {($($arg:tt)*)=>{()};}
extern crate self as embassy_time;
extern crate self as embassy_nrf;
use std::{sync::{Mutex,atomic::{AtomicU32,Ordering::Relaxed}},future::Future};
static CLOCK:AtomicU32=AtomicU32::new(100);
pub struct Instant(u32);impl Instant{pub fn now()->Self{Self(CLOCK.load(Relaxed))}pub fn as_millis(&self)->u64{self.0 as u64}}
pub mod pac{pub struct Power;pub static POWER:Power=Power;impl Power{pub fn usbregstatus(&self)->Self{Power}pub fn read(&self)->Self{Power}pub fn vbusdetect(&self)->bool{false}}}
pub mod input_device{pub mod rotary_encoder{#[derive(Clone,Copy,PartialEq)]pub enum Direction{Clockwise,CounterClockwise,None}}pub mod battery{pub fn current_battery_status()->crate::types::battery::BatteryStatus{crate::types::battery::BatteryStatus::Unavailable}}}
pub mod types{pub use rmk_types::{ble, connection};pub mod battery{pub enum BatteryStatus{Unavailable}}}
pub mod event{use crate::input_device::rotary_encoder::Direction;#[derive(Clone,Copy)]pub struct KeyPos{pub row:u8,pub col:u8}#[derive(Clone,Copy)]pub struct RotaryEncoderPos{pub direction:Direction}#[derive(Clone,Copy)]pub enum KeyboardEventPos{Key(KeyPos),RotaryEncoder(RotaryEncoderPos)}#[derive(Clone,Copy)]pub struct KeyboardEvent{pub pos:KeyboardEventPos,pub pressed:bool}}
pub mod ble{pub mod profile{pub const DONGLE_PROFILE:u8=3;#[derive(Debug)]pub enum BleProfileAction{Switch(u8),ClearBond}}}
pub mod storage{#[derive(Debug)]pub enum FlashOperationMessage{ConnectionType(crate::types::connection::ConnectionType)}}
static LOG:Mutex<Vec<String>>=Mutex::new(Vec::new());
pub mod channel{pub struct Sender;impl Sender{pub async fn send<T:std::fmt::Debug>(&self,value:T){crate::LOG.lock().unwrap().push(format!("{value:?}"));}}pub static FLASH_CHANNEL:Sender=Sender;pub static BLE_PROFILE_CHANNEL:Sender=Sender;}
pub mod state{use crate::types::connection::*;static PREF:std::sync::Mutex<ConnectionType>=std::sync::Mutex::new(ConnectionType::Usb);pub fn set_preferred_connection(v:ConnectionType){*PREF.lock().unwrap()=v;}pub fn current_connection_status()->ConnectionStatus{ConnectionStatus{preferred:*PREF.lock().unwrap(),..ConnectionStatus::new()}}}
#[path="../../rmk-ui-core/rmk/src/panda_ui.rs"]mod menu;
use menu as panda_ui;
#[path="../../rmk-ui-core/rmk/src/panda_lighting.rs"]mod panda_lighting;
#[path="../../rmk-ui-core/rmk/src/panda_wpm.rs"]mod panda_wpm;
#[path="../../rmk-ui-core/rmk/src/panda_matrix.rs"]mod panda_matrix;
#[path="../../rmk-ui-core/rmk/src/host/via/keycode_convert.rs"]mod keycode_convert;
mod features;
#[path="../../rmk-ui-core/rmk/src/panda_colors.rs"]mod panda_colors;
#[path="../../rmk-ui-core/rmk/src/panda_maintenance.rs"]mod panda_maintenance;
#[path="../../rmk-ui-core/rmk/src/panda_reply.rs"]mod panda_reply;
mod rc4;
#[path="../../rmk-ui-core/rmk/src/panda_record.rs"]mod panda_record;
mod persistence;
#[path="../../rmk-ui-core/rmk/src/panda_override.rs"]mod panda_override;
mod overrides;
#[path="../src/display_update.rs"]mod display_update;
mod display;
#[path="../src/assets.rs"]mod assets;
#[path="../src/pixel_font.rs"]mod pixel_font;
#[path="../src/render.rs"]mod render;
fn block_on<T>(f:impl Future<Output=T>)->T{let mut f=std::pin::pin!(f);let w=std::task::Waker::noop();match f.as_mut().poll(&mut std::task::Context::from_waker(w)){std::task::Poll::Ready(v)=>v,_=>panic!("Unexpected blocking in mocked command")}}
fn key(row:u8,col:u8,pressed:bool)->event::KeyboardEvent{event::KeyboardEvent{pos:event::KeyboardEventPos::Key(event::KeyPos{row,col}),pressed}}
fn image(path:&str,f:&render::Frame){let mut bytes=b"P4\n32 128\n".to_vec();for y in 0..128{for byte in 0..4{let mut b=0;for bit in 0..8{let x=byte*8+bit;let on=f.pixels[(y/8)*32+x]&(1<<(y%8))!=0;if !on{b|=1<<(7-bit)}}bytes.push(b);}}std::fs::write(path,bytes).unwrap();}
fn main(){
 assert_eq!(menu::RGB.load(Relaxed),3,"Default must be dim white");
 assert_eq!(block_on(menu::intercept(key(1,1,true))),0);
 // EC11 is fully passed through, including a five-second press.
 for duration in [100,700,4999,5000,8000]{CLOCK.store(1000,Relaxed);assert_eq!(block_on(menu::intercept(key(4,5,true))),0);CLOCK.store(1000+duration,Relaxed);assert_eq!(block_on(menu::intercept(key(4,5,false))),0);assert_eq!(menu::PAGE.load(Relaxed),0);}
 block_on(menu::command(26));assert_eq!(menu::PAGE.load(Relaxed),0,"Legacy hold-toggle cannot restore the removed gesture");
 block_on(menu::command(16));assert_eq!(menu::PAGE.load(Relaxed),1,"Assignable Menu opens immediately on command");
 let rotate=event::KeyboardEvent{pressed:true,pos:event::KeyboardEventPos::RotaryEncoder(event::RotaryEncoderPos{direction:input_device::rotary_encoder::Direction::Clockwise})};assert_eq!(block_on(menu::intercept(rotate)),1);assert_eq!(menu::SELECTION.load(Relaxed),1);
 block_on(menu::command(19));assert!(LOG.lock().unwrap().iter().any(|s|s=="Switch(0)"));assert_eq!(menu::PAGE.load(Relaxed),0);assert_eq!(state::current_connection_status().preferred,types::connection::ConnectionType::Ble);
 block_on(menu::command(16));assert_eq!(block_on(menu::intercept(key(0,0,true))),1);assert_eq!(block_on(menu::intercept(key(0,0,false))),1);assert_eq!(menu::PAGE.load(Relaxed),0);
 for index in 0..5{block_on(menu::command(25));menu::SELECTION.store(index,Relaxed);block_on(menu::command(19));assert_eq!(menu::RGB.load(Relaxed),index);}
 block_on(menu::command(25));menu::SELECTION.store(5,Relaxed);block_on(menu::command(19));assert_eq!(menu::PAGE.load(Relaxed),3);menu::BRIGHT.store(100,Relaxed);block_on(menu::intercept(rotate));assert_eq!(menu::BRIGHT.load(Relaxed),100);menu::rotate(false);assert_eq!(menu::BRIGHT.load(Relaxed),89);
 block_on(menu::command(20));assert_eq!(menu::PAGE.load(Relaxed),2);block_on(menu::command(20));assert_eq!(menu::PAGE.load(Relaxed),1);block_on(menu::command(20));assert_eq!(menu::PAGE.load(Relaxed),0);
 // A click started before opening the menu must still release normally.
 assert_eq!(block_on(menu::intercept(key(9,5,true))),0);block_on(menu::command(16));assert_eq!(block_on(menu::intercept(key(9,5,false))),0);
 // A consumed button press remains consumed if the menu times out before release.
 assert_eq!(block_on(menu::intercept(key(9,5,true))),1);CLOCK.store(50000,Relaxed);menu::timeout();assert_eq!(menu::PAGE.load(Relaxed),0);assert_eq!(block_on(menu::intercept(key(9,5,false))),1);
 block_on(menu::command(16));menu::SELECTION.store(4,Relaxed);block_on(menu::command(19));assert_eq!(state::current_connection_status().preferred,types::connection::ConnectionType::Usb);
 assert_eq!(block_on(menu::intercept(key(4,5,true))),0);assert_eq!(block_on(menu::intercept(key(4,5,false))),0);
 let history=std::array::from_fn(|i|20+(i*3%33)as u8);
 let mut model=render::Model{wpm:42,layer:0,mods:0x22,leds:2,battery:Some(82),usb:false,mode:0,link:2,page:0,selection:0,brightness:20,history:&history};
 for wpm in [0,10,11,59,60,99,100,255]{model.wpm=wpm;for layer in 0..8{model.layer=layer;for left in [true,false]{let f=render::Frame::draw(left,&model,1000);let hw=f.hardware();for y in 0..128{for x in 0..32{let a=f.pixels[(y/8)*32+x]&(1<<(y%8))!=0;let b=hw[(x/8)*128+127-y]&(1<<(x%8))!=0;assert_eq!(a,b,"RC2 orientation changed a pixel");}}}}}
 // A single top-left logical pixel moves to the opposite physical corner vs RC1.
 let mut corner=render::Frame::new();corner.px(0,0,true);let hw=corner.hardware();assert_eq!(hw[127],1);assert_eq!(hw.iter().map(|v|v.count_ones()).sum::<u32>(),1);
 assert!(render::Frame::text_width("DONGLE",1)<=28);
 model.wpm=42;model.layer=0;image("left.pbm",&render::Frame::draw(true,&model,1000));model.battery=Some(64);image("right.pbm",&render::Frame::draw(false,&model,1000));
 model.page=1;model.selection=0;image("menu.pbm",&render::Frame::draw(true,&model,1000));
 for page in 1..=3{for choice in 0..8{model.page=page;model.selection=choice;let _=render::Frame::draw(true,&model,1000);}}
 model.page=0;model.link=1;assert_ne!(render::Frame::draw(true,&model,0).pixels,render::Frame::draw(true,&model,650).pixels,"Connecting must blink");model.link=2;assert_eq!(render::Frame::draw(false,&model,0).pixels,render::Frame::draw(false,&model,650).pixels,"Connected must not blink");
 println!("PASS: 100..8000 ms EC11 presses pass through; only assigned Menu command opens menu; RC2 display is flipped 180 degrees vs RC1; real menu commands, paired button events, brightness, timeout, USB/BLE, 128 frame rotations, menus, connection blink states");
 features::verify();
 rc4::verify();
 persistence::verify();
 overrides::verify();
 display::verify();
}
