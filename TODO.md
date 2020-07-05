# TODO

- Run flamegraph to see where we're spending time (pick a good example set.)
- See if we can remove the `regex` (and `lazy_static`?) dependencies.
- Switch from `serde` to something lighter?
- Check if using `rayon` actually buys us anything, since most of our runtime is probably loading
  from and saving to disk.