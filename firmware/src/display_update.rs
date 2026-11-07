//! Local display animation; BLE still carries only actual WPM changes.
pub struct SmoothWpm{start:u16,target:u16,changed:u64}
impl SmoothWpm{
 pub const fn new()->Self{Self{start:0,target:0,changed:0}}
 pub fn value(&self,now:u64)->u16{let elapsed=now.saturating_sub(self.changed).min(200)as i64;let value=self.start as i64+(self.target as i64-self.start as i64)*elapsed/200;value as u16}
 pub fn update(&mut self,target:u16,now:u64)->u16{if target!=self.target{self.start=self.value(now);self.target=target;self.changed=now;}self.value(now)}
}
pub struct History{pub samples:[u8;30],last:Option<u64>}
impl History{
 pub const fn new()->Self{Self{samples:[0;30],last:None}}
 pub fn update(&mut self,now:u64,wpm:u16){
  let Some(last)=self.last else{self.last=Some(now);return;};let steps=now.saturating_sub(last)/2000;if steps==0{return;}
  self.last=Some(last+steps*2000);
  if steps>=30{self.samples.fill(0);}else{self.samples.copy_within(steps as usize..30,0);self.samples[30-steps as usize..].fill(0);}
  self.samples[29]=wpm.min(255)as u8;
 }
 pub fn live(&self,wpm:u16)->[u8;30]{let mut out=self.samples;out[29]=wpm.min(255)as u8;out}
}
/// Exclusive end; one SSD1306 page is 128 physical columns.
pub fn dirty_span(previous:&[u8],next:&[u8],force:bool)->Option<(usize,usize)>{
 assert_eq!(previous.len(),128);assert_eq!(next.len(),128);
 if force{return Some((0,128));}
 let start=previous.iter().zip(next).position(|(a,b)|a!=b)?;
 let end=previous.iter().zip(next).rposition(|(a,b)|a!=b).unwrap()+1;
 Some((start,end))
}
