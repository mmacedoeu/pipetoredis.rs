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

fn handle_pipe(symbol : String, number : String, con: redis::Connection) -> Result<(), throw::Error<Error>> {
    let name = getname(&symbol, number);
    info!("Pipename: {:?}", name);


    loop {
        
            unsafe {
                match stop_loop {
                    Some(ref z) => if z.load(Ordering::Relaxed) {break},
                    None => {},
                }                
            }

    	let mut a = throw!(NamedPipe::new(&name));
        debug!("pipe new");

        let cp = throw!(CompletionPort::new(1));
        info!("CompletionPort new");
        cp.add_handle(3, &a);
        a.connect();
        info!("connect");
    	let mut over = Overlapped::zero();

        loop {

            unsafe {
                match stop_loop {
                    Some(ref z) => if z.load(Ordering::Relaxed) {break},
                    None => {},
                }                
            }

            let mut data = Vec::with_capacity(4096);

            debug!("Overlapped");
            let result = unsafe {
                data.set_len(4096);
                debug!("read_overlapped");
                a.read_overlapped(&mut data, &mut over)
            };

        // check `result` to see if an error happened
            //throw!(result);
                match result {
                    Err(e) => {error!("{:?}", e); break},
                    Ok(_) => {},
                }               

        // wait for the I/O to complete
            let notification = cp.get(None).unwrap();
            debug!("notification");
            unsafe {
                data.set_len(notification.bytes_transferred() as usize); // update how many bytes were read
            }

            let string = String::from_utf8(data).unwrap(); // parse utf-8 to a string

        // work with string
            trace!("{:?}", string);
            let key = symbol.clone();
            let _ : () = con.lpush(key, string).unwrap();

        }    

    }


    Ok(())
}

fn getname(symbol: &String, number: String) -> String {
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

    let client = redis::Client::open("redis://192.168.122.1/").unwrap();
    let con = client.get_connection().unwrap();

    let mut result = Ok(());
    unsafe {
    	stop_loop = Some(AtomicBool::new(false));
      	signal(2, interrupt);
    	result = handle_pipe(param1, param2, con);
    }
    match result {
        Err(e) => error!("{:?}", e),
        _ => info!("Normal termination !"),
    }

}
