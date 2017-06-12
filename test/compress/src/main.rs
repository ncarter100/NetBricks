extern crate e2d2;

use e2d2::interface::*; // PacketRx / Tx
use e2d2::operators::*; // merge()
use e2d2::scheduler::*; // Sheduler
use std::fmt::Display;

mod nf;

//Display is similar to Debug but for user facing output.
fn test<T, S>(ports: Vec<T>, sched: &mut S)
    where T: PacketRx + PacketTx + Display + Clone + 'static,
          S: Scheduler + Sized
{
    println!("Receiving started");

    let mut pipelines: Vec<_> = ports
        .iter()
        .map(|port| nf::compress(ReceiveBatch::new(port.clone()), sched).send(port.clone()))
        .collect();
    println!("Running {} pipelines", pipelines.len());
    if pipelines.len() > 1 {
        sched.add_task(merge(pipelines)).unwrap()
    } else {
        sched.add_task(pipelines.pop().unwrap()).unwrap()
    };
}

fn main() {
    println!("Hello, world!");
}
