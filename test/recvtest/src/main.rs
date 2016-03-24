#![feature(box_syntax)]
extern crate e2d2;
extern crate time;
extern crate simd;
extern crate getopts;
extern crate rand;
use e2d2::io;
use e2d2::io::*;
use e2d2::headers::*;
use getopts::Options;
use std::env;
use std::cell::Cell;
use std::rc::Rc;
use std::collections::HashMap;

const CONVERSION_FACTOR: u64 = 1000000000;

fn monitor<T: 'static + Batch>(parent: T, recv_cell: Rc<Cell<u32>>)
    -> CompositionBatch {
    let f = box |hdr: &mut MacHeader| {
        let src = hdr.src.clone();
        hdr.src = hdr.dst;
        hdr.dst = src;
    };

    // We need to move the recv_cell Rc cell into g, instead of borrowing.
    let g = box move |_: &MacHeader| {
        recv_cell.set(recv_cell.get() + 1);
    };
    let mut x:usize = 0;

    parent
    .parse::<MacHeader>()
    .filter(box move |_| { 
        x += 1;
        (x % 2) == 0
    } )
    .transform(f)
    .map(g).compose()
}

fn recv_thread(ports: Vec<io::PmdPort>, queue: i32, core: i32) {
    io::init_thread(core, core);
    println!("Receiving started");

    let recv_cell = Rc::new(Cell::new(0));
    let mut pipelines: Vec<CompositionBatch> = ports.iter().map(|port| { monitor(io::ReceiveBatch::new(port.copy(), 
                                                                                queue), recv_cell.clone())} )
                                                           .collect();
    let combined = merge(pipelines.pop().expect("No pipeline"), pipelines.pop().expect("No pipeline")); 
    let mut send_port = ports[1].copy();
    let mut pipeline = combined.send(&mut send_port, queue);

    let mut cycles = 0;
    let mut rx = 0;
    let mut no_rx = 0;
    let mut start = time::precise_time_ns() / CONVERSION_FACTOR;
    loop {
        recv_cell.set(0);
        pipeline.process();
        let recv = recv_cell.get();
        rx += recv;
        cycles += 1;
        if recv == 0 {
            no_rx += 1
        }
        let now = time::precise_time_ns() / CONVERSION_FACTOR;
        if now > start {
            println!("{} rx_core {} pps {} no_rx {} loops {}",
                     (now - start),
                     core,
                     rx,
                     no_rx,
                     cycles);
            rx = 0;
            no_rx = 0;
            cycles = 0;
            start = now;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optmulti("w", "whitelist", "Whitelist PCI", "PCI");
    opts.optmulti("c", "core", "Core to use", "core");
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("m", "master", "Master core", "master");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print!("{}", opts.usage(&format!("Usage: {} [options]", program)));
    }
    let cores_str = matches.opt_strs("c");
    let master_core = matches.opt_str("m")
                             .unwrap_or_else(|| String::from("0"))
                             .parse()
                             .expect("Could not parse master core spec") ;
    println!("Using master core {}", master_core);

    let whitelisted = matches.opt_strs("w");
    if cores_str.len() > whitelisted.len() {
        println!("More cores than ports");
        std::process::exit(1);
    }
    let cores: Vec<i32> = cores_str.iter()
                                   .map(|n: &String| n.parse().ok().expect(&format!("Core cannot be parsed {}", n)))
                                   .collect(); 
    for (core, wl) in cores.iter().zip(whitelisted.iter()) {
        println!("Going to use core {} for wl {}", core, wl);
    }
    let mut core_map = HashMap::<i32, Vec<i32>>::with_capacity(cores.len());
    for (core, port) in cores.iter().zip(0..whitelisted.len()) {
        {
            match core_map.get(&core) {
                Some(_) => {core_map.get_mut(&core).expect("Incorrect logic").push(port as i32)},
                None => {core_map.insert(core.clone(), vec![port as i32]); ()}
            }
        }
    }

    io::init_system_wl(&format!("recv{}", cores_str.join("")),
                       master_core,
                       &whitelisted);
    let mut thread: Vec<std::thread::JoinHandle<()>> = core_map.iter()
                                                            .map(|(core, ports)| {
                                                                let c = core.clone();
                                                                let recv_ports:Vec<PmdPort> = 
                                                                    ports.iter()
                                                                         .map(|p| io::PmdPort::new_mq_port(p.clone() as i32,
                                                                                                           1,
                                                                                                           1,
                                                                                                           &vec![c],
                                                                                                           &vec![c])
                                                                                                .unwrap()).collect();
                                                                std::thread::spawn(move || recv_thread(recv_ports, 
                                                                                                       0, c))
                                                            })
                                                            .collect();
    let _ = thread.pop().expect("No cores started").join();
}