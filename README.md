# `tree++`: A Much Better Windows `tree` Command

*[中文](./README-zh.md)*

The `tree` command on Windows has barely changed since its release nearly 40 years ago. In today's LLM era, it is frequently used to describe project structures, yet the built-in options (`/f` and `/a`) are clearly insufficient. At the same time, it is slow.

**`tree++` is a comprehensive upgrade to `tree`**, introducing the following improvements for the Windows platform:

- ***Full compatibility with the original Windows `tree` command parameters and output, while extending the instruction set to include features such as file size display, recursion depth limits, output style modifications, result export to files, and exclusion of specified directories (including `.gitignore` support)***
- ***Multithreading support for exponential performance improvements in large and complex directories; when run with administrator privileges, MFT mode is available for extremely high speed***

Supports traditional Windows-style options (e.g., `/f`, case-insensitive) and Unix-style options (e.g., `-f` and `--files`).

**`tree++` is implemented in `Rust`**, open-sourced on [GitHub](https://github.com/Water-Run/treepp).

## Installation

Download `tree++.zip` from [Release](https://github.com/Water-Run/treepp/releases/tag/0.1.0), extract it to a suitable directory, and add that directory to your PATH environment variable.

Open Windows Terminal and run:

```powershell
treepp /v
```

You should see:

```powershell
tree++ version 0.1.0

A Better tree command for Windows.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

Installation is complete.

After that, you can use it just like the standard Windows `tree` command:

```powershell
treepp /f
```

For large directories, you can use MFT: directly scanning the NTFS directory index is extremely fast. This requires running `treepp` with administrator privileges; using `Sudo for Windows` is recommended. `treepp` will automatically detect the current execution mode:

```powershell
sudo treepp /f --mft
```

> Note: In `mft` mode, some features will be limited

## Quick Overview

| Option Set (Equivalent Forms) | Description                                              |
|-------------------------------|----------------------------------------------------------|
| `--help` `-h` `/?`            | Show help information                                    |
| `--version` `-v` `/V`         | Show version information                                 |
| `--ascii` `-a` `/A`           | Draw the tree using ASCII characters                     |
| `--files` `-f` `/F`           | Show files                                               |
| `--full-path` `-p` `/FP`      | Show full paths                                          |
| `--human-readable` `-H` `/HR` | Show file sizes in human-readable units                  |
| `--no-indent` `-i` `/NI`      | Hide tree connector lines                                |
| `--reverse` `-r` `/R`         | Reverse sort order                                       |
| `--size` `-s` `/S`            | Show file sizes (bytes)                                  |
| `--date` `-d` `/DT`           | Show last modified date                                  |
| `--exclude` `-I` `/X`         | Exclude matched files                                    |
| `--level` `-L` `/L`           | Limit recursion depth                                    |
| `--include` `-m` `/M`         | Show only matched files                                  |
| `--quote` `-q` `/Q`           | Wrap file names in double quotes                         |
| `--dirs-first` `-D` `/DF`     | List directories before files                            |
| `--disk-usage` `-u` `/DU`     | Show cumulative directory size                           |
| `--ignore-case` `-c` `/IC`    | Case-insensitive matching                                |
| `--no-report` `-n` `/NR`      | Hide the summary report at the end                       |
| `--prune` `-P` `/P`           | Prune empty directories                                  |
| `--sort` `-S` `/SO`           | Specify sorting (`name`, `size`, `mtime`, etc.)          |
| `--no-header` `-N` `/NH`      | Hide volume information and header report                |
| `--silent` `-l` `/SI`         | Silent mode (used with the `output` option)              |
| `--output` `-o` `/O`          | Save output to a file (`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T`          | Number of scan threads (default: 24)                     |
| `--mft` `-M` `/MFT`           | Use MFT (requires admin privileges, limited features)    |
| `--gitignore` `-g` `/G`       | Follow `.gitignore`                                      |

> For the complete option set, see: [tree++ Options Documentation](./OPTIONS.md)
