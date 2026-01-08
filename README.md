# `tree++`: A Much Better Windows `tree` Command

*[中文](./README-zh.md)*

The `tree` command on Windows has seen virtually no changes since it was released nearly 40 years ago. In today’s LLM era, as a tool that is very commonly used to describe project structure, the only two options—`/f` and `/a`—are clearly insufficient. It is also not particularly fast.

**`tree++` is a comprehensive upgrade to `tree`**, bringing the following to the Windows `tree` command:

* ***An expanded parameter set, covering common features such as displaying file sizes, limiting recursion depth, changing output style, writing results to a file, and excluding specified directories (including honoring `.gitignore`)***
* ***Better performance via a Rust implementation; additionally supports multithreading in batch mode, delivering a significant scanning speed improvement***
* ***Full compatibility with the original Windows `tree` command’s parameters and output format, while also supporting Unix-style options (such as `-f` and `--files`)***

**`tree++` is implemented in `Rust`** and is open-sourced on [GitHub](https://github.com/Water-Run/treepp).


*Performance comparison (using `C:\Windows` as an example):*

| Type                       | Time (`ms`) | Multiplier |
|----------------------------|------------:|-----------:|
| `tree /f` (Windows Native) |  `34367.81` |    `1.00x` |
| `treepp /f`                |   `8948.63` |    `3.84x` |
| `treepp /f /nb`            |   `8690.36` |    `3.95x` |
| `treepp /f /nb /b`         |   `3816.34` |    `9.01x` |
| `treepp /f /nb /b /t 1`    |  `10672.62` |    `3.22x` |
| `treepp /f /nb /b /t 2`    |   `6769.22` |    `5.08x` |
| `treepp /f /nb /b /t 4`    |   `4717.16` |    `7.29x` |
| `treepp /f /nb /b /t 8`    |   `3797.09` |    `9.05x` |
| `treepp /f /nb /b /t 16`   |   `3026.32` |   `11.36x` |
| `treepp /f /nb /b /t 32`   |   `3013.44` |   `11.40x` |

## Installation

Download `tree++.zip` from [Release](https://github.com/Water-Run/treepp/releases/tag/0.1.0), extract it to a suitable directory, and add that directory to your environment variables.

Open Windows Terminal and run:

```powershell
treepp /v
```

You should see output like:

```plaintext
tree++ version 0.1.0

A Much Better Windows `tree` Command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

Installation is now complete.

After that, you can use it the same way as the normal Windows `tree` command:

```powershell
treepp /f
```

## Quick Reference

| Option Set (Equivalent Forms) | Description                                                 |
|-------------------------------|-------------------------------------------------------------|
| `--help` `-h` `/?`            | Show help information                                       |
| `--version` `-v` `/V`         | Show version information                                    |
| `--ascii` `-a` `/A`           | Draw the tree using ASCII characters                        |
| `--files` `-f` `/F`           | Show files                                                  |
| `--full-path` `-p` `/FP`      | Show full paths                                             |
| `--human-readable` `-H` `/HR` | Show file sizes in human-readable form                      |
| `--no-indent` `-i` `/NI`      | Do not show tree connector lines                            |
| `--reverse` `-r` `/R`         | Sort in reverse order                                       |
| `--size` `-s` `/S`            | Show file size (bytes)                                      |
| `--date` `-d` `/DT`           | Show last modified date                                     |
| `--exclude` `-I` `/X`         | Exclude matching files                                      |
| `--level` `-L` `/L`           | Limit recursion depth                                       |
| `--include` `-m` `/M`         | Show only matching files                                    |
| `--disk-usage` `-u` `/DU`     | Show cumulative directory size                              |
| `--report` `-e` `/RP`         | Show trailing summary statistics                            |
| `--prune` `-P` `/P`           | Prune empty directories                                     |
| `--no-win-banner` `-N` `/NB`  | Hide the Windows-native tree banner output                  |
| `--silent` `-l` `/SI`         | Silent terminal output (use with `output`)                  |
| `--output` `-o` `/O`          | Output results to a file (`.txt`, `.json`, `.yml`, `.toml`) |
| `--batch` `-b` `/B`           | Use batch mode                                              |
| `--thread` `-t` `/T`          | Number of scan threads (batch mode, default is 8)           |
| `--gitignore` `-g` `/G`       | Honor `.gitignore`                                          |

> For the full option set, see: [tree++ Options Documentation](./OPTIONS.md)