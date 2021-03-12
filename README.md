# actix-threadpool-diesel

> Integrate Diesel into Actix Web via a threadpool cleanly and efficiently.

This has the same API as [tokio-diesel] except that it uses a threadpool instead
of [`tokio::task::block_in_place`], which cannot be run from single threaded
executors like the Actix Web runtime.

**The credit for this library goes to the [contributors] of [tokio-diesel].**

## Usage

See [the example](./examples/simple.rs) for detailed usage information.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[contributors]: https://github.com/mehcode/tokio-diesel/graphs/contributors
[tokio-diesel]: https://github.com/mehcode/tokio-diesel
[`actix_web::web::block`]: https://docs.rs/actix-web/3.3.2/actix_web/web/fn.block.html
[`tokio::task::block_in_place`]: https://docs.rs/tokio/1.3.0/tokio/task/fn.block_in_place.html
