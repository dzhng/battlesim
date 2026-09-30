#[derive(Clone,Debug,PartialEq)]pub enum RefinementStatus{Pending,Complete(Option<f64>)}
pub struct Refinement{a:V2,b:V2,m:Mobility,policy:RoutePolicy,pub work:usize}
impl Refinement{
 pub fn new(a:V2,b:V2,m:Mobility,policy:RoutePolicy)->Self{Self{a,b,m,policy,work:0}}
 pub fn poll(&mut self,grid:&NavGrid,budget:&mut usize)->RefinementStatus{if *budget==0{return RefinementStatus::Pending;}*budget-=1;self.work+=1;RefinementStatus::Complete(grid.segment_cost(self.a,self.b,&self.m,self.policy))}
}
