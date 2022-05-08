# rustr

R rust port

## Goal

* Modernize C based R code base using Rust
* Reimplement `src/main`
* Do not touch performance-critical code such as math functions written in Fortran. 

## Status

* Only front-end is ported in Rust.
* R will be incrementally ported. 

## Build

```bash
# set R 4.2.0
echo "4.2.0" > R_VERSION

# build
cargo build
```
