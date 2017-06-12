// compress network function
//
// compress / uncompress the L3 payload of a packet

use e2d2::headers::*; // NullHeader
use e2d2::operators::*;
use e2d2::scheduler::*; // Sheduler

pub fn compress<T: 'static + Batch<Header = NullHeader>>(parent: T, _s: &mut Scheduler) -> CompositionBatch {
    println!("njc compress called");
    let pipeline = parent;
    pipeline.compose()
}
