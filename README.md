# The parallel gzip

This is an implementation of a parallel gzip. It works by splitting the input
into chunks (currently by 100MBs, but this will be configurable in the future).
Each chunk is compressed independently and the results are concatenated
together. Such result can be read and decompressed by the usual gzip
implementation.

The motivation is to speed up transfers of large amounts of data across a fast
network through ssh. The ssh throughput is limited by either its compression or
encryption routines, which are single-threaded. This allows turning compression
of in ssh and using multiple cores to compress the data. As the decompression
is much faster, it is not necessary to use parallel *decompression*.

## Limitations

There are certain limitations:

* It is not configurable. This is because it is in an early stage of implementation.
* The compressed representation is slightly different than from the usual
  sequential gzip. Technically, the output is multiple concatenated gzips, but
  decompression tools commonly accepts that. Furthermore, due to the
  independent chunks, the compression ratio is likely to be a bit worse.
* It uses more memory, to buffer the chunks.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.

