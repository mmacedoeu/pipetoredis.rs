extern crate csv;
extern crate redis;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate miow;
#[macro_use] extern crate throw;

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

fn handle_pipe(name : &String) -> Result<(), throw::Error<Error>> {
	let mut a = throw!(NamedPipe::new(name));
    debug!("pipe new");
    let name2 = name.clone();
    let t = thread::spawn(move || {
	    let mut f = File::create(name2);
        debug!("pipe created");
    });

    let cp = throw!(CompletionPort::new(1));
    debug!("CompletionPort new");
    cp.add_handle(3, &a);
    a.connect();
    debug!("connect");
    let mut data = Vec::with_capacity(4096);
	let mut over = Overlapped::zero();

    debug!("Overlapped");
    let result = unsafe {
        data.set_len(4096);
        debug!("read_overlapped");
        a.read_overlapped(&mut data, &mut over)
    };

// check `result` to see if an error happened
    throw!(result);

// wait for the I/O to complete
    let notification = cp.get(None).unwrap();
    info!("notification");
    unsafe {
        data.set_len(notification.bytes_transferred() as usize); // update how many bytes were read
    }

    let string = String::from_utf8(data).unwrap(); // parse utf-8 to a string

// work with string
    info!("{:?}", string);

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

    let mut result = Ok(());
    unsafe {
    	stop_loop = Some(AtomicBool::new(false));
      	signal(2, interrupt);
    	result = handle_pipe(&pipe_name);
    }
    match result {
        Err(e) => error!("{:?}", e),
        _ => info!("Normal termination !"),
    }

}
