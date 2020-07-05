# TODO

- Run flamegraph to see where we're spending time (pick a good example set.)
- Switch from `serde` to something lighter?
- Check if using `rayon` actually buys us anything, since most of our runtime is probably loading
  from and saving to disk.
- Work on errors - they aren't very descriptive (they should include filepaths and names of
  tracks.)
- Have a command to auto extract album art (maybe during generate.)
- Add track setting to override album art path.
- Examine migrating into multiple subcrates (text, manifest, lib, bin?)
- Detect filename / title discrepancy on generate.
- Come up with a better name.
