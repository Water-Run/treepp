# `tree++`: A Better `tree` Command for Windows

*[中文](./README-zh.md)*

The Windows `tree` command has barely changed since it was released nearly 40 years ago. In the LLM era, it is frequently used to describe project structures, yet with only two parameters—`/f` and `/a`—its functionality is clearly limited. It is also not particularly fast.

`tree++` is a comprehensive upgrade to `tree`, open-sourced on [GitHub](https://github.com/Water-Run/treepp). It includes:

* Fully compatible with the original Windows `tree` command, while extending the option set with commonly needed features such as showing file sizes, limiting recursion depth, and exporting results to a file
* Supports traditional Windows-style options (e.g., `/f`, case-insensitive), Unix-style options (e.g., `-f` and `--files`), and PowerShell-style options (e.g., `-Files`)
* Supports multithreading, making it easy to achieve exponential performance improvements in large and complex directories; when running in administrator mode, it can also scan the MFT directly to achieve extremely fast speeds.

`tree++` is implemented in `Rust`.

## Installation

Download `tree++.zip` from [GitHub Release](https://github.com/Water-Run/treepp/releases/tag/1.0.0), extract it to an appropriate directory, and add that directory to your environment variables (PATH).

Open Windows Terminal and run:

```powershell
treepp /v
```

If it prints:

```powershell
tree++ version 0.1.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

then the installation is complete.

After that, you can use it just like the standard Windows `tree` command:

```powershell
treepp /f
```

For large directories, you can use MFT: directly scanning the NTFS directory index is extremely fast. This requires running `treepp` with administrator privileges; using `Sudo for Windows` is recommended. `treepp` will automatically detect the current execution state:

```powershell
sudo treepp /f
```

## Quick Reference

| Option Set (Equivalent Forms)                  | Description                                                 |
| ---------------------------------------------- | ----------------------------------------------------------- |
| `--help` `-h` `/?` `-Help`                     | Show help information                                       |
| `--version` `-v` `/V` `-Version`               | Show version information                                    |
| `--ascii` `-a` `/A` `-Ascii`                   | Draw the tree using ASCII characters                        |
| `--files` `-f` `/F` `-Files`                   | Show files                                                  |
| `--full-path` `-p` `/FP` `-FullPath`           | Show full paths                                             |
| `--human-readable` `-H` `/HR` `-HumanReadable` | Display file sizes in human-readable format                 |
| `--no-indent` `-i` `/NI` `-NoIndent`           | Do not show tree connector lines                            |
| `--reverse` `-r` `/R` `-Reverse`               | Sort in reverse order                                       |
| `--size` `-s` `/S` `-Size`                     | Show file size (bytes)                                      |
| `--date` `-d` `/DT` `-Date`                    | Show last modified time                                     |
| `--exclude` `-I` `/X` `-Exclude`               | Exclude files that match the pattern                        |
| `--level` `-L` `/L` `-Level`                   | Limit recursion depth                                       |
| `--include` `-m` `/M` `-Include`               | Only show files that match the pattern                      |
| `--quote` `-q` `/Q` `-Quote`                   | Wrap filenames in double quotes                             |
| `--dirs-first` `-D` `/DF` `-DirsFirst`         | List directories before files                               |
| `--disk-usage` `-u` `/DU` `-DiskUsage`         | Show cumulative directory size                              |
| `--ignore-case` `-c` `/IC` `-IgnoreCase`       | Ignore case when matching                                   |
| `--no-report` `-n` `/NR` `-NoReport`           | Do not show summary statistics at the end                   |
| `--prune` `-P` `/P` `-Prune`                   | Prune empty directories                                     |
| `--sort` `-S` `/SO` `-Sort`                    | Specify sort mode (`name`, `size`, `mtime`, etc.)           |
| `--no-header` `-N` `/NH` `-NoHeader`           | Do not show volume info and header report                   |
| `--silent` `-l` `/SI` `-Silent`                | Silent terminal output (used with the `output` option)      |
| `--output` `-o` `/O` `-Output`                 | Export results to a file (`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T` `-Thread`                 | Number of scanning threads (default: 24)                    |
| `--no-mft` `-nm` `/NM` `-NoMFT`                              | Force disabling MFT usage (instead of automatic switching in administrator mode) |

> For the full option list, see: [tree++ Options Documentation](./OPTIONS.md)
