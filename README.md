# `tree++`: A Better `tree` Command for Windows

*[English](./README.md)*

The `tree` command on Windows has seen almost no changes since its release nearly 40 years ago. In the LLM era, it is a frequently used tool for describing project structures, yet it still offers only two parameters (`/f` and `/a`), which is clearly insufficient for modern needs.

`tree++` is a comprehensive upgrade to `tree`, open-sourced on [GitHub](https://github.com/Water-Run/treepp). It includes:

* While remaining compatible with the original Windows `tree` command behavior, it extends the option set with commonly needed features such as file size display and recursion depth limits.
* Supports traditional Windows-style options (e.g., `/f`, case-insensitive), Unix-style options (e.g., `-f`), and PowerShell-style options (e.g., `-Files`).
* Faster, especially in large and complex directories.

`tree++` is implemented in `Rust`.

## Installation

Download `tree++.zip` from [GitHub Release](https://github.com/Water-Run/treepp/releases/tag/1.0.0), extract it to a suitable directory, and add that directory to your environment variables (`PATH`).

Open Windows Terminal and run:

```powershell
treepp /v
```

You should see:

```powershell
tree++ version 0.1.0
link: https://github.com/Water-Run/treepp
```

Installation is now complete.

After that, you can use it just like the standard Windows `tree` command:

```powershell
treepp /f
```

## Options Quick Reference

| Option Set (Equivalent Forms)                  | Description                                       |
| ---------------------------------------------- | ------------------------------------------------- |
| `--help` `-h` `/?` `/H` `-Help`                | Show help information                             |
| `--version` `-v` `/V` `-Version`               | Show version information                          |
| `--ascii` `-A` `/A` `-Ascii`                   | Draw the tree using ASCII characters              |
| `--files` `-f` `/F` `-Files`                   | Show files                                        |
| `--full-path` `-p` `/FP` `-FullPath`           | Show full paths                                   |
| `--human-readable` `-H` `/HR` `-HumanReadable` | Show file sizes in a human-readable format        |
| `--no-indent` `-i` `/NI` `-NoIndent`           | Do not show tree connector lines                  |
| `--reverse` `-r` `/R` `-Reverse`               | Sort in reverse order                             |
| `--size` `-s` `/S` `-Size`                     | Show file size (bytes)                            |
| `--date` `-D` `/DT` `-Date`                    | Show last modified date                           |
| `--exclude` `-I` `/X` `-Exclude`               | Exclude matching files                            |
| `--level` `-L` `/L` `-Level`                   | Limit recursion depth                             |
| `--include` `-P` `/M` `-Include`               | Only show matching files                          |
| `--quote` `-Q` `/Q` `-Quote`                   | Wrap file names in double quotes                  |
| `--dirs-first` `-O` `/O` `-DirsFirst`          | Show directories first                            |
| `--du` `-u` `/DU` `-DiskUsage`                 | Show cumulative directory size                    |
| `--ignore-case` `-iC` `/IC` `-IgnoreCase`      | Ignore case when matching                         |
| `--no-report` `-N` `/NR` `-NoReport`           | Do not show the summary report at the end         |
| `--prune` `-P` `/P` `-Prune`                   | Prune empty directories                           |
| `--sort` `-S` `/SORT` `-Sort`                  | Specify sort mode (`name`, `size`, `mtime`, etc.) |
| `--no-header` `-NH` `/NH` `-NoHeader`          | Do not show volume info and header report         |
| `--silent` `-SI` `/SI` `-Silent`               | Produce no output (use with `output`)             |
| `--output` `-o` `/OUT` `-Output`               | Output results (`.txt`, `.json`, `.yml`, `.toml`) |

> For the complete option set, see: [tree++ Options Documentation](./OPTIONS.md)
