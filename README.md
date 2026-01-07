# `tree++`: A Much Better Windows `tree` Command

*[中文](./README-zh.md)*

The Windows `tree` command has barely changed since it was released nearly 40 years ago. In the LLM era, it is a very commonly used tool for describing project structures, but with only `/f` and `/a`, its capabilities are clearly insufficient. It is also slow.

**`tree++` is a comprehensive upgrade to `tree`**, bringing the following to the Windows `tree` command:

- ***An extended option set, covering commonly used features such as showing file sizes, limiting recursion depth, changing output styles, exporting results to a file, and excluding specific directories (including honoring `.gitignore`).***
- ***Multi-threading support, delivering significant performance improvements on large and complex directories.***
- ***Fully compatible with the original Windows `tree` command’s options and output, and also compatible with Unix-style options (such as `-f` and `--files`).***

**`tree++` is implemented in `Rust`**, and is open-sourced on [GitHub](https://github.com/Water-Run/treepp).

*Performance comparison (using `C:\Windows` as an example):*

| Type                          | Time (`ms`) | Multiplier |
| ----------------------------- | ----------- | ---------- |
| Native `tree`                 | `34055.50`  | 1.0x       |
| `treepp` (default, 8 threads) | `3480.12`   | 9.79x      |
| `treepp` (1 thread)           | `6687.58`   | 5.09x      |

## Installation

Download `tree++.zip` from [Release](https://github.com/Water-Run/treepp/releases/tag/0.1.0), extract it to a suitable directory, and add that directory to your environment variables.

Open Windows Terminal and run:

```powershell
treepp /v
```

You should see output like:

```plaintext
tree++ version 0.1.0

A much better Windows tree command.

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
| `--ignore-case` `-c` `/IC`    | Ignore case when matching                                   |
| `--report` `-e` `/RP`         | Show trailing summary statistics                            |
| `--prune` `-P` `/P`           | Prune empty directories                                     |
| `--sort` `-S` `/SO`           | Specify sort method (`name`, `size`, `mtime`, etc.)         |
| `--no-win-banner` `-N` `/NB`  | Hide the Windows-native tree banner output                  |
| `--silent` `-l` `/SI`         | Silent terminal output (use with `output`)                  |
| `--output` `-o` `/O`          | Output results to a file (`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T`          | Number of scan threads (default is 8)                       |
| `--gitignore` `-g` `/G`       | Honor `.gitignore`                                          |
| `--quote` `-q` `/Q`           | Wrap file names in double quotes                            |
| `--dirs-first` `-D` `/DF`     | Display directories first                                   |

> For the full option set, see: [tree++ Options Documentation](./OPTIONS.md)
