extern crate csv;
extern crate redis;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate miow;

use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use redis::{Commands, PipelineCommands};
use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use std::fs::{File, OpenOptions};
use std::sync::mpsc::channel;
use std::io::{BufReader,BufRead};
use std::io::Error;

use miow::pipe::{NamedPipe, NamedPipeBuilder};
use miow::iocp::CompletionPort;
use miow::Overlapped;

extern "C" {
  fn signal(sig: u32, cb: extern fn(u32));
}

extern fn interrupt(_:u32) {
	unsafe {
		stop_loop.as_ref().map(|z| z.store(true, Ordering::Relaxed));
	}
}

static mut stop_loop : Option<AtomicBool> = None;

fn handle_pipe(name : &String) -> Result<(), Error> {
	let mut a = try!(NamedPipe::new(name));
    let name2 = name.clone();
    let t = thread::spawn(move || {
	    let mut f = File::create(name2);
    });

    let cp = try!(CompletionPort::new(1));
    cp.add_handle(3, &a);
    a.connect();     
    let mut b : String = String::new();
	let mut over = Overlapped::zero();
    unsafe {
    	a.read_overlapped(&mut b.as_ref(), &mut over);
    }
    let status = cp.get(None);       
    info!("{:?}", b);

	t.join();
    Ok(())
}

fn name(symbol: String, number: String) -> String {
    format!(r"\\.\pipe\{}{}", symbol, number)
}

fn main() {

	let param1 = ::std::env::args().nth(1).unwrap();
	let param2 = ::std::env::args().nth(2).unwrap();

    let format = |record: &LogRecord| {
        format!("{} - {}", record.level(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

    let pipe_name = name(param1, param2);
    info!("Pipename: {:?}", &pipe_name);

    unsafe {
    	stop_loop = Some(AtomicBool::new(false));
      	signal(2, interrupt);
    	handle_pipe(&pipe_name).unwrap();      	
    }
}
