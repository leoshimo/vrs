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

It hopes to combine powerful ideas from those systems into one cohesive project,
so I can build rich personal software ecosystems and superplatforms at the speed
of thought.

Its key principles are: 

- simplicity
- progress
- joy

## Status

vrs is a sandbox project, focused on play and experimentation in a pure-fun,
pure-utility environment. While I live-on vrs everyday, the platform is very
volatile in both concepts and implementation. Be warned!

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsd`: A runtime implementation as a system daemon
- `lyric`: Embedded Lisp Dialect and Virtual Machine
- `vrsctl`: A thin CLI client over `libvrs`
- `vrsjmp`: A GUI launch bar client
