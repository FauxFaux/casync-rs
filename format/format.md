# Files involved

 * `.catar` -> archive containing a directory tree (like "tar")
 * `.caidx`-> index file referring to a directory tree (i.e. a `.catar` file)
 * `.caibx`-> index file referring to a blob (i.e. any other file)
 * `.castr` -> chunk store directory (where we store chunks under their hashes)
 * `.cacnk` -> a compressed chunk in a chunk store (i.e. one of the files stored below a `.castr` directory)

# Conventions

 * `leu64`: 64-bit unsigned integer, stored in little endian format as eight octets

# .catar

Series of packets.

## Packet format

 * `leu64` length, inclusive of this header
 * `leu64` magic number (see `format.rs`)
 * {packet data}

## Packets

### `Entry`

`length` must be precisely 64 bytes; eight eight-byte integers,
i.e. a catar file will start with `4000 0000 0000 0000 51bb 5bea bcfa 9613`.

 1. `leu64` feature flags
 2. `leu64` mode
 3. `leu64` flags
 4. `leu64` uid
 5. `leu64` gid
 6. `leu64` mtime

### `User`, `Group`

These are string records. The entire payload is taken as a string.
It is null-terminated.

e.g. length = 21, -8 -8 for the header -> 5 bytes. payload: `faux\0`.

### `Filename`

Also a string record.

If we're inside an `Entry`, set the `Entry`'s filename. If we're not,
push another directory onto the filename stack.