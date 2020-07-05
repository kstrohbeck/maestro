# Songmaster

Songmaster is a command line application that helps you keep your music
organized. It manages naming and tagging of mp3 files based on human-readable
manifests.

## Quick Links

- [Change Log](./CHANGELOG.md)
- [User Guide](./GUIDE.md)

## Basic usage

If you have a folder of MP3 files for a given album you want to manage the tags
of, you can run

```
songmaster generate
```

to generate an "extras" directory containing an "album.yaml" file. This file
is where you'll define the info about the album. The file will automatically
contain as much information as songmaster was able to extract from your mp3s,
and you can edit it however you like to add more - it's a standard
[YAML](https://yaml.org/) file.

To rename your mp3s to canonical filenames, you can run

```
songmaster rename
```

Currently, this doesn't edit the manifest, so you'll have to go in and remove
the old filenames.

When you're ready to update your tags, run

```
songmaster update
```

and your mp3s will be updated.

## Covers

...
