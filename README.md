# VRS

[![Rust](https://github.com/leoshimo/vrs/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/leoshimo/vrs/actions/workflows/rust.yml)

🚧 Under Heavy Construction

## What is this?

VRS is a WIP personal software runtime, inspired by Emacs, Erlang, Unix, Plan 9,
and Hypermedia systems.

The intention is to incorporate great ideas from each of those systems into one,
cohesive experience in service of my personal happiness.

Core principles are simplicity and joy that allows individuals to build
rich software - [Renaissance tech for renaissance people](https://web.archive.org/web/20210428062809/https://twitter.com/dhh/status/1341758748717510659).

## Status

VRS is currently in **design phase**. The runtime primitives are under rapid
iteration and experimentation.

Focus on key areas, such as security and performance, are deferred for the time being.

## Structure

- `libvrs`: The `vrs` library crate shared by runtime and client implementations
- `vrsd`: A runtime implementation as a system daemon
- `vrsctl`: A thin CLI client over `libvrs`
- `lemma`: Embedded Lisp Dialect and Bytecode VM

## Logging

Set `RUST_LOG` to configure logging level. Default is `info`

```sh
# logging level to debug
$ RUST_LOG=debug cargo run --bin vrsd
```

# Mapping the Project in Latent Space

> In the multiverse, you can live up to your ultimate potential. We discovered a
> way to temporarily link your consciousness to another version of yourself,
> accessing all of their memories and skills.
>
> It's called verse jumping.
>
> — Alpha Waymond

> The experience of `emacs` everywhere

> If `xdg-open`, `systemd`, and `dbus` raised a child in an alternative universe
> where Lisp replaced XML as dominant configuration language

> The thing about ideas, is that ideas start out small... tiny and weak and
> fragile... Ideas need an environment where creator can nurture them.. feed
> them and shape their growth - B.V.
