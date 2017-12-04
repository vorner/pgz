extern crate failure;
extern crate flate2;
extern crate futures;
extern crate futures_cpupool;
extern crate num_cpus;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::io::{self, Read, Write};
use std::process;
use std::thread::{self, JoinHandle};

use failure::Error;
use flate2::Compression;
use flate2::bufread::GzEncoder;
use futures::{Future, Stream, Sink};
use futures::future;
use futures::sync::mpsc::{self, Receiver, Sender};
use futures_cpupool::CpuPool;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    #[structopt(short = "j", long = "jobs", help = "How many parallel jobs should be run")]
    jobs: Option<usize>,
    #[structopt(short = "c", long = "chunk-size",
                help = "The size of one chunk, before it gets processed",
                default_value = "33554432")]
    chunk: usize,
    #[structopt(short = "q", long = "queue-size", help = "Maximum number of waiting chunks",
                default_value = "2")]
    queue: usize,
    #[structopt(short = "O", long = "compression", help = "The compression level, 0-9",
                default_value = "3")]
    compression: u32,
}

fn read_input(mut sink: Sender<Vec<u8>>, chunk_size: usize) -> JoinHandle<Result<(), Error>> {
    thread::spawn(move || -> Result<(), Error> {
        let stdin_unlocked = io::stdin();
        let mut stdin = stdin_unlocked.lock();
        loop {
            let mut limit = (&mut stdin).take(chunk_size as u64);
            let mut buffer = Vec::with_capacity(chunk_size);
            limit.read_to_end(&mut buffer)?;
            if buffer.is_empty() {
                return Ok(()); // A real EOF
            }
            sink = sink.send(buffer).wait()?;
        }
    })
}

fn write_output(stream: Receiver<Vec<u8>>) -> JoinHandle<Result<(), Error>> {
    thread::spawn(move || -> Result<(), Error> {
        stream.map_err(|()| failure::err_msg("Error on channel"))
            .for_each(|chunk| {
                let stdout_unlocked = io::stdout();
                let mut stdout = stdout_unlocked.lock();
                stdout.write_all(&chunk).map_err(Error::from)
            }).wait()
    })
}

fn compress(i: &[u8], level: Compression) -> Vec<u8> {
    // Pre-allocate space for all the data, compression is likely to make it smaller.
    let mut result = Vec::with_capacity(i.len());
    let mut gz = GzEncoder::new(&i[..], level);
    gz.read_to_end(&mut result).unwrap();
    result
}

fn run() -> Result<(), Error> {
    let options = Options::from_args();
    let (in_sender, in_receiver) = mpsc::channel(options.queue);
    let (out_sender, out_receiver) = mpsc::channel(1);
    let in_thread = read_input(in_sender, options.chunk);
    let out_thread = write_output(out_receiver);

    let jobs = options.jobs.unwrap_or_else(num_cpus::get);
    let pool = CpuPool::new(jobs);
    let compression = options.compression;
    in_receiver
        .map_err(|()| failure::err_msg("Error on channel"))
        .map(|chunk| {
             pool.spawn(future::lazy(move || {
                 future::ok(compress(&chunk, Compression::new(compression)))
             }))
        })
        .buffered(num_cpus::get())
        .forward(out_sender)
        .wait()?;

    out_thread.join().unwrap()?;
    in_thread.join().unwrap()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
