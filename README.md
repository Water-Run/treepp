# `tree++`: A Better `tree` Command for Windows

*[中文](./README-zh.md)*

The Windows `tree` command has barely changed since it was released nearly 40 years ago. In the LLM era, `tree` is frequently used to describe project structure, and the built-in options (`/f` and `/a`) are clearly not enough.

`tree++` is a comprehensive upgrade to `tree`, open-sourced on [GitHub](https://github.com/Water-Run/treepp). It provides:

* Extended options while remaining compatible with the original Windows `tree` behavior (e.g., file size display, recursion depth limit, etc.)
* Support for classic Windows/CMD style (e.g. `/f`, case-insensitive), Unix style (e.g. `-f` / `--files`), and PowerShell style (e.g. `-Files`) options
* Better performance, especially in large and complex directories

`tree++` is implemented in `Zig` (version: `0.16.0`).

## Installation

Download `tree++.zip` from [GitHub Release](https://github.com/Water-Run/treepp/releases/tag/1.0.0), extract it to an appropriate directory, and add that directory to your `PATH`.

Open Windows Terminal and run:

```powershell
treepp /v
```

You should see:

```powershell
tree++ version 1.0.0
link: https://github.com/Water-Run/treepp
```

Installation is complete.

After that, you can use it the same way as the standard Windows `tree` command:

```powershell
treepp /f
```

## Options Overview

| Option Set (Equivalent Forms)                  | Description                                       |
| ---------------------------------------------- | ------------------------------------------------- |
| `--help` `-h` `/?` `/H` `-Help`                | Show help information                             |
| `--version` `-v` `/V` `-Version`               | Show version information                          |
| `--ascii` `-A` `/A` `-Ascii`                   | Draw the tree using ASCII characters              |
| `--files` `-f` `/F` `-Files`                   | Show files                                        |
| `--full-path` `-p` `/FP` `-FullPath`           | Show full paths                                   |
| `--human-readable` `-H` `/HR` `-HumanReadable` | Show file sizes in human-readable units           |
| `--no-indent` `-i` `/NI` `-NoIndent`           | Hide tree connector lines                         |
| `--reverse` `-r` `/R` `-Reverse`               | Reverse sort order                                |
| `--size` `-s` `/S` `-Size`                     | Show file sizes (bytes)                           |
| `--date` `-D` `/DT` `-Date`                    | Show last modified time                           |
| `--exclude` `-I` `/X` `-Exclude`               | Exclude matched files                             |
| `--level` `-L` `/L` `-Level`                   | Limit recursion depth                             |
| `--include` `-m` `/M` `-Include`               | Show only matched files                           |
| `--quote` `-Q` `/Q` `-Quote`                   | Wrap file names in double quotes                  |
| `--dirs-first` `-O` `/O` `-DirsFirst`          | List directories before files                     |
| `--disk-usage` `--du` `-u` `/DU` `-DiskUsage`  | Show cumulative directory size                    |
| `--ignore-case` `-iC` `/IC` `-IgnoreCase`      | Case-insensitive matching                         |
| `--no-report` `-N` `/NR` `-NoReport`           | Hide the summary report at the end                |
| `--prune` `-P` `/P` `-Prune`                   | Prune empty directories                           |
| `--sort` `-S` `/SORT` `-Sort`                  | Specify sorting (`name`, `size`, `mtime`, etc.)   |
| `--no-header` `-NH` `/NH` `-NoHeader`          | Hide volume info and header report                |
| `--silent` `-SI` `/SI` `-Silent`               | Produce no output (used with the `output` option) |
| `--save` `-sv` `/SV` `-Save`                   | Save output (`.txt`, `.json`, `.yml`, `.toml`)    |

> For the complete option set, see: [tree++ Options Documentation](./OPTIONS.md)
