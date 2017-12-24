# Files involved

 * `.catar` -> archive containing a directory tree (like "tar")
 * `.caidx`-> index file referring to a directory tree (i.e. a `.catar` file)
 * `.caibx`-> index file referring to a blob (i.e. any other file)
 * `.castr` -> chunk store directory (where we store chunks under their hashes)
 * `.cacnk` -> a compressed chunk in a chunk store (i.e. one of the files stored below a `.castr` directory)

# Conventions

 * `leu64`: 64-bit unsigned integer, stored in little endian format as eight octets

# .catar

A `.catar` is a series of packets, which represent a directory structure.

Everything is stored in alphabetical order, and traversed depth first.
A directory structure like the following:

```
$ (cd two; tree .)
.
├── b
│   ├── three
│   └── two
└── one
```

..will be stored like this:

```
$ casync mtree two.catar | cut ...
.       type=dir
b       type=dir
b/three type=file
b/two   type=file
one     type=file
```

The directory structure is represented by a state machine which works nothing
like how I expect. Here's an attempt at representing it:

```
Entry: dir: true
  Name: b, in entry, depth now 1
  Entry: dir: true
    Name: three, in entry, depth now 2
    Entry: dir: false
      Data
    Name: two, outside entry, depth still 2
    Entry: dir: false
      Data
    Bye: leaving entry, depth now 1
  Name: one, outside entry, depth still 1
  Entry: dir: false
    Data
  Bye: leaving entry, depth now 0
```

Rules:

 * A `Data` record causes us to leave the current `Entry`, it was a file.
 * On processing a `Name` record, if were are "inside" and `Entry`, then
    we are inside a directory. Otherwise, we're specifying the name for the
    next item.
 * `Bye` causes us to leave a directory. When we leave the root directory,
    we are done.


Maybe it would be easier to think of it as:

```
item: name: . [this record is implicit]
  item-metadata: dir: true
  item: name: b
    item-metadata: dir: true
    item: name: three
      item-metadata: dir: false
      item-data; end of item
    item: name: two
      item-metadata: dir: false
      item-data; end of item
    item-end; without any data
  item: name: one
    item-metadata: dir: false
    item-data; end of item
  item-end; without any data
```

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