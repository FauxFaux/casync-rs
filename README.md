# casync-rs

A simple, safe, library implementation of [`casync`](https://github.com/systemd/casync), in Rust.


## WARNING

There are practically no tests, and barely any error checking in this code.


## What is `casync`?

`casync` is a series of layers, which can be used to build something
like a backup tool, or a docker registry, or a git storage backend;
anything which needs snapshots of directories. It uses *magic* to
get some level of deduplication without the massive overheads of some
other techniques.


The layers:

 1. real filesystem -> virtual filesystem (ordering, resolution, picking metadata)
 2. virtual filesystem -> tar-like stream (called a `catar`)
 3. any stream -> bunch of files (called `chunks`) and an `index`


So, given an index:

 1. fetch the appropriate `chunks` named in the `index`
 2. transform the `chunks` back into a stream
 3. if the stream was a `catar`, unpack it back to the filesystem 


## What can `casync-rs` do?

Read:

 - [ ] download an `index` and the appropriate `chunk` files
 - [x] convert an `index` and `chunks` into a stream
 - [x] pick files out a `catar`
 - [ ] support unix extensions in `catar` (e.g. symlinks)
 - [ ] unpack a `catar` to the filesystem


Write:

 - [ ] convert an actual filesystem into a virtual filesystem
 - [ ] convert a virtual filesystem into a `catar`
 - [ ] convert a stream into `chunks` and an `index`
 - [ ] upload anything

## License

MIT or Apache-2.0.
