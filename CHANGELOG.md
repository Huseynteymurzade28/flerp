# Changelog

## 0.7.0 — 2026-08-17

### The Files pane

flerp held one file and gave you no way to reach the next one. Run it on a
directory — or on nothing at all, which now means the working directory — and a
tree pane stands beside every mode:

```bash
flerp                # the working directory
flerp ./contracts    # somewhere else
```

- Folders are read only when opened, so the pane costs what is on screen: a
  directory of a hundred thousand nested files opens as fast as an empty one.
- Committing a search with `Enter` sweeps every text file and PDF under the
  root. Each row carries its match count, and a closed folder carries the total
  hiding inside it, so you can see where the matches are before opening
  anything.
- Picking a file out of a swept tree lands the viewer on its first match.
- The loaded file is marked `●` in the tree.
- `f` moves the keyboard between the tree and the mode beside it. `Ctrl-w` does
  the same, but several terminal emulators bind it to "close tab", so it is not
  the one the footer advertises.
- `Enter` opens a folder or loads a file, `h`/`l` and the arrow keys fold, and
  clicking a row picks it while a second click on the same row opens it.
- `flerp --search needle` with no path now opens the tree with the sweep already
  run, rather than complaining that it has nothing to search.

### Directory-wide search

`--search` now takes directories and several paths, not just one file:

```bash
flerp --search "invoice" ./contracts
flerp --search "invoice" --json ./contracts *.pdf
```

Every text file and every PDF under the named paths is searched, and matches
print in grep's `file:line:text` format. This is the thing `grep -r` cannot do:
a folder of PDFs is opaque to every line-oriented tool in the shell.

- `--json` groups matches by file and reports the page each one landed on, along
  with what was searched and how many files were looked at.
- The walk skips hidden files, binaries (a NUL byte in the first 8 KiB, the same
  test grep uses) and images, and does not follow symlinked directories.
- A file named on the command line is searched whether or not the walk's filter
  would have picked it up.
- Results are sorted by path, so the same directory reports the same way twice.
- Unreadable directories and files that fail to open are reported rather than
  dropped: under `--json` in a `skipped` array, otherwise on stderr, so stdout
  stays parseable.
- Embedded images are no longer decoded when only the text is needed, so a
  sweep across illustrated PDFs costs only its text.

### Changed

- The positional argument is now `[PATH]...` rather than a single path. Naming
  more than one path skips the TUI, and so does a directory with `--search`,
  which is a grep and belongs on stdout. A directory on its own opens the tree.
- `--text` and `--extract-images` say they need a single file instead of picking
  one from a set.
- Error messages no longer repeat the program name after the `flerp: ` prefix.
- Library API: `Cli::file` is now `Cli::paths`, and `HeadlessRequest::file` is
  now `HeadlessRequest::targets`.

### Fixed

- The README pointed `flerp-git` users at a `master` branch that does not exist;
  the branch is `main`.

## Earlier releases

0.6.0 and before are recorded in the git history rather than here; this file
starts at 0.7.0.
