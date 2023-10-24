# VRS

[![Rust](https://github.com/leoshimo/vrs/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/leoshimo/vrs/actions/workflows/rust.yml)

🚧 Under Heavy Construction

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

## What is this?

VRS is a WIP personal software runtime, inspired by Emacs, Erlang, Unix, Plan 9,
and Hypermedia systems. 

Its key principles are simplicity and joy.

## Status

VRS is currently in **design phase**. The runtime primitives are under rapid
iteration and experimentation.

Focus on key areas, such as security and performance, are deferred for the time being.

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsd`: A runtime implementation as a system daemon
- `vrsctl`: A thin CLI client over `libvrs`
- `lyric`: Embedded Lisp Dialect and Bytecode VM

## Logging

Set `RUST_LOG` to configure logging level. Default is `info`

```sh
# logging level to debug
$ RUST_LOG=debug cargo run --bin vrsd
```
