// SPDX-License-Identifier: GPL-2.0-or-later
// Approved 32x128 design, with the preserved PandaKB/QMK pet and layer assets.
use super::{assets,pixel_font::glyph};
use core::fmt::Write;
use super::types::{ble::BleState, connection::{ConnectionStatus, ConnectionType}};

pub fn connection_badge(status: ConnectionStatus, central_reachable: bool) -> (u8, u8) {
 let active = central_reachable.then(|| status.decide_active()).flatten();
 let mode = match active.unwrap_or(status.preferred) {
  ConnectionType::Usb => 4,
  ConnectionType::Ble if status.ble.profile == 3 => 0,
  ConnectionType::Ble => status.ble.profile.min(2) + 1,
 };
 let link = if active.is_some() {
  2
 } else if central_reachable && mode != 4 && status.ble.state == BleState::Advertising {
  1
 } else {
  0
 };
 (mode, link)
}
pub struct Model<'a>{pub wpm:u16,pub layer:u8,pub mods:u8,pub leds:u8,pub battery:Option<u8>,pub usb:bool,pub mode:u8,pub link:u8,pub page:u8,pub selection:u8,pub brightness:u8,pub history:&'a[u8;30]}
pub struct Frame{pub pixels:[u8;512]}
impl Frame{
 pub fn new()->Self{Self{pixels:[0;512]}}
 pub fn px(&mut self,x:i16,y:i16,on:bool){if !(0..32).contains(&x)||!(0..128).contains(&y){return;}let index=(y as usize/8)*32+x as usize;let mask=1<<(y%8);if on{self.pixels[index]|=mask}else{self.pixels[index]&=!mask}}
 pub fn rect(&mut self,x:i16,y:i16,w:i16,h:i16,on:bool){for yy in y..y+h{for xx in x..x+w{self.px(xx,yy,on);}}}
 pub fn line(&mut self,mut x:i16,mut y:i16,x2:i16,y2:i16){let dx=(x2-x).abs();let sx=if x<x2{1}else{-1};let dy=-(y2-y).abs();let sy=if y<y2{1}else{-1};let mut e=dx+dy;loop{self.px(x,y,true);if x==x2&&y==y2{break}let e2=e*2;if e2>=dy{e+=dy;x+=sx}if e2<=dx{e+=dx;y+=sy}}}
 pub fn text_width(t:&str,scale:i16)->i16{(t.chars().map(|ch|(glyph(ch).0+1)*scale).sum::<i16>()-scale).max(0)}
 pub fn text(&mut self,t:&str,mut x:i16,y:i16,scale:i16,on:bool){for ch in t.chars(){let(w,rows)=glyph(ch);for (j,row) in rows.iter().enumerate(){for i in 0..w{if row&(1<<(w-1-i))!=0{self.rect(x+i*scale,y+j as i16*scale,scale,scale,on);}}}x+=(w+1)*scale;}}
 pub fn center(&mut self,t:&str,y:i16,scale:i16){let x=(32-Self::text_width(t,scale))/2;self.text(t,x,y,scale,true);}
 pub fn badge(&mut self,t:&str,y:i16,on:bool){self.rect(0,y,32,9,true);self.rect(1,y+1,30,7,false);if on{self.rect(2,y+2,28,5,true);}let x=(32-Self::text_width(t,1))/2;self.text(t,x,y+2,1,!on);}
 fn battery(&mut self,m:&Model){if m.usb{self.center("USB",0,1);return;}if let Some(value)=m.battery{self.rect(0,0,9,6,true);self.rect(1,1,7,4,false);self.rect(9,2,1,2,true);let fill=(value as i16*5/100).max(0);self.rect(2,2,fill,2,true);let mut text=heapless::String::<5>::new();write!(text,"{}%",value).ok();self.text(&text,12,0,1,true);}else{self.center("--%",0,1);}}
 fn pet(&mut self,m:&Model,tick:u64){let data=if m.mods&0x22!=0{&assets::BARK}else if m.mods&0x11!=0{&assets::SNEAK}else if m.wpm<=10{&assets::SIT}else if m.wpm<60{&assets::WALK}else{&assets::RUN};let frame=if m.wpm<=10&&m.mods&0x33==0{1}else{((tick/200)%2)as usize};for (i,b) in data[frame*96..frame*96+96].iter().enumerate(){for bit in 0..8{let yy=(i/32)*8+bit;if yy<22&&b&(1<<bit)!=0{self.px((i%32)as i16,7+yy as i16,true);}}}}
 fn layer(&mut self,layer:u8){let data=match layer{0=>&assets::BASE,1=>&assets::RAISE,2=>&assets::LOWER,6=>{self.badge("NAV",34,true);return;},7=>{self.badge("MOUSE",34,true);return;},_=>&assets::ADJUST};for(i,b)in data.iter().enumerate(){for bit in 0..8{let yy=(i/18)*8+bit;if(3..20).contains(&yy)&&b&(1<<bit)!=0{self.px(7+(i%18)as i16,31+(yy-3)as i16,true);}}}}
 fn connection(&mut self,m:&Model,tick:u64){let name=match m.mode{0=>"DONGLE",1=>"BT1",2=>"BT2",3=>"BT3",_=>"USB"};let on=m.link==2||(m.link==1&&(tick/600)%2==0);self.badge(name,116,on);}
 fn menu(&mut self,m:&Model,tick:u64){self.pet(m,tick);if m.page==3{self.center("BRIGHT",38,1);let mut s=heapless::String::<5>::new();write!(s,"{}%",m.brightness).ok();self.center(&s,55,2);self.center("BACK",86,1);return;}
 self.center(if m.page==1{"LINK"}else{"RGB"},31,1);
 let choices:&[&str]=if m.page==1{&["DONGLE","BT1","BT2","BT3","USB","RGB","PAIR","BACK"]}else{&["GUIDE","LAYER","WAVE","WHITE","OFF","BRIGHT","BACK"]};for(i,t)in choices.iter().enumerate(){self.badge(t,38+i as i16*11,m.selection as usize==i);}}
 pub fn draw(left:bool,m:&Model,tick:u64)->Self{let mut f=Self::new();f.battery(m);if left&&m.page!=0{f.menu(m,tick);return f;}
 if left{f.pet(m,tick);f.layer(m.layer);for(i,label)in["WIN","ALT","CTRL","SHIFT","CAPS"].iter().enumerate(){let on=match i{0=>m.mods&0x88!=0,1=>m.mods&0x44!=0,2=>m.mods&0x11!=0,3=>m.mods&0x22!=0,_=>m.leds&2!=0};f.badge(label,50+i as i16*11,on);}}
 else{f.center("WPM",12,1);let mut s=heapless::String::<5>::new();write!(s,"{:02}",m.wpm.min(999)).ok();f.center(&s,22,if m.wpm>=100{2}else{3});f.text("120",1,42,1,true);f.line(1,49,1,95);f.line(1,95,30,95);let mut last_y=95;for(i,value)in m.history.iter().enumerate(){let y=95-(*value as i16).min(120)*46/120;if i>0{f.line(i as i16,last_y,i as i16+1,y);}last_y=y;}f.text("0",1,98,1,true);f.text("60S",1,105,1,true);f.text("0",28,105,1,true);}
 f.connection(m,tick);f}
 /// Rotate into SSD1306 pages, flipped 180 degrees from UI RC1 for the mounted panels.
 pub fn hardware(&self)->[u8;512]{let mut out=[0;512];for y in 0..128{for x in 0..32{if self.pixels[(y/8)*32+x]&(1<<(y%8))!=0{let px=127-y;let py=x;out[(py/8)*128+px]|=1<<(py%8);}}}out}
}
