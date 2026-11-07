//! Shared lighting implementation for OLED menu, VialRGB packets and RGB keycodes.
use core::sync::atomic::{AtomicU8,Ordering::Relaxed};
use rmk_types::action::LightAction;
use crate::panda_ui::{RGB,BRIGHT};
pub static HUE:AtomicU8=AtomicU8::new(0);
pub static SAT:AtomicU8=AtomicU8::new(255);
pub static SPEED:AtomicU8=AtomicU8::new(128);
static LAST_ON:AtomicU8=AtomicU8::new(0);
pub const SAT_LEVELS:[u8;10]=[0,28,57,85,113,142,170,198,227,255];
pub const BRIGHTNESS_LEVELS: [u8; 10] = [0, 11, 22, 33, 44, 56, 67, 78, 89, 100];

pub fn adjust_brightness(increase: bool) {
    let value = BRIGHT.load(Relaxed);
    let next = if increase {
        BRIGHTNESS_LEVELS.iter().copied().find(|level| *level > value).unwrap_or(100)
    } else {
        BRIGHTNESS_LEVELS.iter().rev().copied().find(|level| *level < value).unwrap_or(0)
    };
    BRIGHT.store(next, Relaxed);
}
pub fn saturation_step(value:u8,increase:bool)->u8{
 if increase{SAT_LEVELS.iter().copied().find(|v|*v>value).unwrap_or(255)}
 else{SAT_LEVELS.iter().copied().rev().find(|v|*v<value).unwrap_or(0)}
}

pub fn from_keycode(code:u16)->Option<LightAction>{
 use LightAction::*;
 Some(match code {0x7800=>BacklightOn,0x7801=>BacklightOff,0x7802=>BacklightToggle,0x7803=>BacklightDown,0x7804=>BacklightUp,0x7805=>BacklightStep,0x7806=>BacklightToggleBreathing,
 0x7820=>RgbTog,0x7821=>RgbModeForward,0x7822=>RgbModeReverse,0x7823=>RgbHui,0x7824=>RgbHud,0x7825=>RgbSai,0x7826=>RgbSad,0x7827=>RgbVai,0x7828=>RgbVad,0x7829=>RgbSpi,0x782a=>RgbSpd,0x782b=>RgbModePlain,0x782c=>RgbModeBreathe,0x782d=>RgbModeRainbow,0x782e=>RgbModeSwirl,_=>return None})
}
pub fn to_keycode(action:LightAction)->Option<u16>{(0x7800..=0x782e).find(|code|from_keycode(*code)==Some(action))}
pub fn toggle(){let old=RGB.load(Relaxed);if old==4{RGB.store(LAST_ON.load(Relaxed),Relaxed);}else{LAST_ON.store(old,Relaxed);RGB.store(4,Relaxed);}}
pub fn step(next:bool){let old=RGB.load(Relaxed).min(7);RGB.store(if next{(old+1)%8}else{(old+7)%8},Relaxed);}
pub fn action(a:LightAction){
 use LightAction::*;
 match a{
  BacklightOn=>{if RGB.load(Relaxed)==4{RGB.store(LAST_ON.load(Relaxed),Relaxed);}},
  BacklightOff=>{let old=RGB.swap(4,Relaxed);if old!=4{LAST_ON.store(old,Relaxed);}},
  BacklightToggle|RgbTog=>toggle(),BacklightStep|RgbModeForward=>step(true),RgbModeReverse=>step(false),
  BacklightUp|RgbVai=>adjust_brightness(true),
  BacklightDown|RgbVad=>adjust_brightness(false),
  RgbHui|RgbHud=>{HUE.store(if a==RgbHui{HUE.load(Relaxed).wrapping_add(8)}else{HUE.load(Relaxed).wrapping_sub(8)},Relaxed);if RGB.load(Relaxed)<=4{RGB.store(5,Relaxed);}},
  RgbSai|RgbSad=>{let value=if RGB.load(Relaxed)==3{0}else{SAT.load(Relaxed)};SAT.store(saturation_step(value,a==RgbSai),Relaxed);if RGB.load(Relaxed)<=4{RGB.store(5,Relaxed);}},
  RgbSpi|RgbSpd=>{SPEED.store(if a==RgbSpi{SPEED.load(Relaxed).saturating_add(8)}else{SPEED.load(Relaxed).saturating_sub(8)},Relaxed);},
  RgbModePlain=>{RGB.store(5,Relaxed);},RgbModeBreathe|BacklightToggleBreathing=>{RGB.store(6,Relaxed);},RgbModeRainbow=>{RGB.store(7,Relaxed);},RgbModeSwirl=>{RGB.store(2,Relaxed);},_=>{}
 }
}
/// Handle the exact 32-byte packets used by Vial 0.7.5. Unsupported writes return 0xff.
pub fn packet(request:&[u8;32],reply:&mut[u8;32]){
 match (request[0],request[1]){
  (0x08,0x40)=>{reply[2..].fill(0);reply[2]=1;reply[4]=255;},
  (0x08,0x41)=>{reply[2..].fill(0);let mode:u16=match RGB.load(Relaxed){4=>0,0=>3,2=>14,6=>6,7=>13,_=>2};reply[2..4].copy_from_slice(&mode.to_le_bytes());reply[4]=SPEED.load(Relaxed);reply[5]=HUE.load(Relaxed);reply[6]=if RGB.load(Relaxed)==3{0}else{SAT.load(Relaxed)};reply[7]=((BRIGHT.load(Relaxed)as u16*255+50)/100)as u8;},
  (0x08,0x42)=>{reply[2..].fill(255);let after=u16::from_le_bytes([request[2],request[3]]);let mut pos=2;for mode in [2u16,3,6,13,14]{if mode>after{reply[pos..pos+2].copy_from_slice(&mode.to_le_bytes());pos+=2;}}},
  (0x07,0x41)=>{let mode=u16::from_le_bytes([request[2],request[3]]);let mapped=match mode{0=>4,2=>5,3=>0,6=>6,13=>7,14=>2,_=>{reply[0]=255;return;}};RGB.store(mapped,Relaxed);SPEED.store(request[4],Relaxed);HUE.store(request[5],Relaxed);SAT.store(request[6],Relaxed);BRIGHT.store(((request[7]as u16*100+127)/255)as u8,Relaxed);},
  // Settings are deliberately volatile in this RC; don't acknowledge persistence.
  _=>{reply[0]=255;}
 }
}
fn hue(h:u8)->[u8;3]{let region=h/43;let rem=(h%43)as u16*6;match region{0=>[255,rem as u8,0],1=>[(255-rem)as u8,255,0],2=>[0,255,rem as u8],3=>[0,(255-rem)as u8,255],4=>[rem as u8,0,255],_=>[255,0,(255-rem)as u8]}}
pub fn color(mode:u8,bright:u8,layer:u8,tick:u64,index:usize)->[u8;3]{
 const GROUP:[u8;36]=[255,255,255,255,255,255,255,4,0,0,0,0,0,0,0,0,4,4,1,1,1,1,2,2,2,2,4,4,3,3,3,3,3,3,3,3];
 const PALETTE:[[u8;3];5]=[[185,160,196],[212,179,107],[128,170,167],[149,180,123],[199,214,194]];
 const LAYERS:[[u8;3];8]=[[180,210,255],[255,130,50],[80,190,255],[210,100,255],[100,230,120],[255,110,170],[240,210,70],[170,160,255]];
 let speed=80-SPEED.load(Relaxed)as u64*75/255;
 let mut value=match mode{0=>if GROUP[index]<5{PALETTE[GROUP[index]as usize]}else{[0;3]},1=>LAYERS[(layer as usize).min(7)],2=>hue(((tick/speed)as u8).wrapping_add((index as u8).wrapping_mul(7))),3=>[255;3],5|6=>hue(HUE.load(Relaxed)),7=>hue((tick/speed)as u8),8=>crate::panda_colors::color(layer,index,tick),_=>[0;3]};
 if matches!(mode,2|5|6|7){let s=SAT.load(Relaxed)as u32;value=value.map(|v|(255-(255-v as u32)*s/255)as u8);}
 let factor=if mode==6{let phase=(tick/(speed*4)%510)as u32;if phase<255{phase}else{510-phase}}else{255};
 value.map(|v|(v as u32*bright.min(100)as u32*48*factor/(255*100*255))as u8)
}
