[![Rust](https://github.com/leoshimo/vrs/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/leoshimo/vrs/actions/workflows/rust.yml)

<p align="center">
    <img width="360" src="https://raw.github.com/leoshimo/vrs/main/assets/vrs.png">
</p>

🚧 Under Heavy Construction

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

## What is this?

[vrs](https://github.com/leoshimo/vrs) is a WIP personal software runtime,
inspired by Emacs, Erlang, Unix, Plan 9, and Hypermedia systems.

Its key principles are simplicity and joy.

## Status

vrs is in early design stage. The runtime primitives are under rapid iteration
and experimentation!

Focus on key areas, such as security and performance, are deferred for now.

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsd`: A runtime implementation as a system daemon
- `vrsctl`: A thin CLI client over `libvrs`
- `lyric`: Embedded Lisp Dialect and Virtual Machine
