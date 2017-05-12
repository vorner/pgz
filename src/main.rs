extern crate flate2;
extern crate futures;
extern crate futures_cpupool;
extern crate num_cpus;
extern crate tokio_core;

use std::thread::{self, JoinHandle};
use std::io::{self, Read, Write};

use flate2::Compression;
use flate2::bufread::GzEncoder;
use futures::{Future, Stream, Sink};
use futures::future;
use futures::sync::mpsc::{self, Receiver, Sender};
use futures_cpupool::CpuPool;
use tokio_core::reactor::Core;

// 100 MB
const CHUNK_SIZE: usize = 100 * 1024 * 1024;

fn read_input(mut sink: Sender<Vec<u8>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdin_unlocked = io::stdin();
        let mut stdin = stdin_unlocked.lock();
        loop {
            let mut limit = (&mut stdin).take(CHUNK_SIZE as u64);
            let mut buffer = Vec::with_capacity(CHUNK_SIZE);
            limit.read_to_end(&mut buffer).unwrap();
            if buffer.is_empty() {
                return; // A real EOF
            }
            sink = sink.send(buffer).wait().unwrap();
        }
    })
}

fn write_output(stream: Receiver<Vec<u8>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdout_unlocked = io::stdout();
        let mut stdout = stdout_unlocked.lock();
        stream.for_each(move |chunk| {
            stdout.write_all(&chunk).map_err(|e| panic!("{}", e))
        }).wait().unwrap();
    })
}

fn compress(i: Vec<u8>, level: Compression) -> Vec<u8> {
    // Pre-allocate space for all the data, compression is likely to make it smaller.
    let mut result = Vec::with_capacity(i.len());
    let mut gz = GzEncoder::new(&i[..], level);
    gz.read_to_end(&mut result).unwrap();
    result
}

fn main() {
    // TODO: Configurable amount
    let (in_sender, in_receiver) = mpsc::channel(5);
    let (out_sender, out_receiver) = mpsc::channel(1);
    let in_thread = read_input(in_sender);
    let out_thread = write_output(out_receiver);

    let pool = CpuPool::new_num_cpus();
    let process = in_receiver
        .map(|chunk| {
             pool.spawn(future::lazy(move || Ok(compress(chunk, Compression::Fast))))
        })
        .buffered(num_cpus::get())
        .forward(out_sender.sink_map_err(|e| panic!("{}", e)));
    let mut core = Core::new().unwrap();
    core.run(process).unwrap();

    out_thread.join().unwrap();
    in_thread.join().unwrap();
}
