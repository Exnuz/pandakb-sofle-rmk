use super::display_update::*;
pub fn verify(){
 let mut smooth=SmoothWpm::new();assert_eq!(smooth.update(100,1000),0);
 let values:[u16;5]=core::array::from_fn(|i|smooth.value(1000+i as u64*50));assert_eq!(values,[0,25,50,75,100]);
 assert_eq!(smooth.update(20,1200),100);assert_eq!(smooth.value(1300),60);assert_eq!(smooth.value(1400),20);
 smooth.update(120,1400);let before=smooth.value(1450);assert_eq!(smooth.update(40,1450),before,"New BLE sample must not jump the displayed value");assert_eq!(smooth.value(1650),40);
 let mut history=History::new();history.update(0,0);history.update(100,40);assert!(history.samples.iter().all(|n|*n==0));assert_eq!(history.live(40)[29],40,"Live endpoint must update before the two-second bucket");
 history.update(2000,20);assert_eq!(history.samples[29],20);history.update(8000,50);assert_eq!(history.samples[26],20);assert_eq!(&history.samples[27..],&[0,0,50],"Missed buckets must not invent a plateau");history.update(100000,7);assert!(history.samples[..29].iter().all(|n|*n==0));assert_eq!(history.samples[29],7);
 let old=[0u8;128];assert_eq!(dirty_span(&old,&old,false),None);assert_eq!(dirty_span(&old,&old,true),Some((0,128)));
 for index in 0..128{let mut next=old;next[index]=255;assert_eq!(dirty_span(&old,&next,false),Some((index,index+1)));}
 for changed in 0..512{
  let mut physical=[0u8;512];let mut next=physical;next[changed]=0xa5;
  let mut transferred=0;for page in 0..4{let base=page*128;if let Some((start,end))=dirty_span(&physical[base..base+128],&next[base..base+128],false){transferred+=end-start;physical[base+start..base+end].copy_from_slice(&next[base+start..base+end]);}}
  assert_eq!(physical,next);assert_eq!(transferred,1,"One-byte change must not resend a 512-byte frame");
 }
 println!("PASS display: 50 ms local WPM animation, continuous retargeting, immediate graph endpoint, 60-second history and missed buckets, all 512 dirty-byte transfers and forced full refresh");
}
