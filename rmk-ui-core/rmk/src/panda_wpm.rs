//! Eight 250 ms buckets: responsive WPM with a two-second sliding window.
pub struct Estimator {samples:[u8;8],next:usize,used:u8}
impl Estimator {
 pub const fn new()->Self{Self{samples:[0;8],next:0,used:0}}
 pub fn sample(&mut self,keys:u8)->u16{
  self.samples[self.next]=keys;self.next=(self.next+1)%8;self.used=(self.used+1).min(8);
  let sum:u32=self.samples.iter().map(|n|*n as u32).sum();
  (sum*60*1000/(5*self.used.max(2)as u32*250))as u16
 }
}
