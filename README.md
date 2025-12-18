# `tree++`: A Better `tree` Command for Windows

*[中文](./README-zh.md)*

The `tree` command on Windows has barely changed since its release nearly 40 years ago. In today’s LLM era, it is frequently used to describe project structures, yet the built-in options (`/f` and `/a`) are clearly insufficient. At the same time, it is slow.

**`tree++` is a comprehensive upgrade to `tree`**, open-sourced on [GitHub](https://github.com/Water-Run/treepp). It provides:

- Extended functionality on top of full compatibility with the original Windows `tree` command, including file size display, recursion depth limits, output to files, and more  
- Support for classic Windows-style options (e.g. `/f`, case-insensitive), Unix-style options (e.g. `-f` and `--files`), and PowerShell-style options (e.g. `-Files`)  
- Exponential performance improvements via multithreading in large and complex directories; when run with administrator privileges, it can also scan the NTFS MFT for extremely high speed  

`tree++` is implemented in Rust.

## Installation

Download `tree++.zip` from the [Release](https://github.com/Water-Run/treepp/releases/tag/0.1.0), extract it to a suitable directory, and add that directory to your PATH.

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

For large directories, you can use MFT scanning: directly scanning the NTFS directory index is extremely fast. This requires running `treepp` with administrator privileges; using `Sudo for Windows` is recommended. `treepp` will automatically detect the current execution mode:

```powershell
sudo treepp /f
```

## Quick Overview

| Option Set (Equivalent Forms)                               | Description                                             |
| ----------------------------------------------------------- | ------------------------------------------------------- |
| `--help` `-h` `/?` `-Help`                                  | Show help information                                   |
| `--version` `-v` `/V` `-Version`                            | Show version information                                |
| `--ascii` `-a` `/A` `-Ascii`                                | Draw the tree using ASCII characters                    |
| `--files` `-f` `/F` `-Files`                                | Show files                                              |
| `--full-path` `-p` `/FP` `-FullPath`                        | Show full paths                                         |
| `--human-readable` `-H` `/HR` `-HumanReadable`              | Show file sizes in human-readable units                 |
| `--no-indent` `-i` `/NI` `-NoIndent`                        | Hide tree connector lines                               |
| `--reverse` `-r` `/R` `-Reverse`                            | Reverse sort order                                      |
| `--size` `-s` `/S` `-Size`                                  | Show file sizes (bytes)                                 |
| `--date` `-d` `/DT` `-Date`                                 | Show last modified date                                 |
| `--exclude` `-I` `/X` `-Exclude`                            | Exclude matched files                                   |
| `--level` `-L` `/L` `-Level`                                | Limit recursion depth                                   |
| `--include` `-m` `/M` `-Include`                            | Show only matched files                                 |
| `--quote` `-q` `/Q` `-Quote`                                | Wrap file names in double quotes                        |
| `--dirs-first` `-D` `/DF` `-DirsFirst`                      | List directories before files                           |
| `--disk-usage` `-u` `/DU` `-DiskUsage`                      | Show cumulative directory size                          |
| `--ignore-case` `-c` `/IC` `-IgnoreCase`                    | Case-insensitive matching                               |
| `--no-report` `-n` `/NR` `-NoReport`                        | Hide the summary report at the end                      |
| `--prune` `-P` `/P` `-Prune`                                | Prune empty directories                                 |
| `--sort` `-S` `/SO` `-Sort`                                 | Specify sorting (`name`, `size`, `mtime`, etc.)         |
| `--no-header` `-N` `/NH` `-NoHeader`                        | Hide volume information and header report               |
| `--silent` `-l` `/SI` `-Silent`                             | Silent mode (used with the `output` option)             |
| `--output` `-o` `/O` `-Output`                              | Save output to a file (`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T` `-Thread`                              | Number of scan threads (default: 24)                    |
| `--no-mft` `-nm` `/NM` `-NoMFT`                             | Force disabling MFT usage (instead of automatic switching in admin mode) |

> For the complete option set, see: [tree++ Options Documentation](./OPTIONS.md)
