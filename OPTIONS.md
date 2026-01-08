# `tree++`: Complete Options Documentation and Examples

This document provides a comprehensive description of all options supported by [tree++](https://github.com/Water-Run/treepp) along with usage examples.

## Mock Directory

The example outputs in this document are based on this mock directory:

```powershell
PS D:\Rust\tree++> treepp /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

> `treepp /F` behaves exactly like the native Windows `tree /F`: displays volume header information and tree structure. Running `treepp` directly also maintains the original semantics of showing only the directory structure.

## General Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`: Optional, defaults to current directory. When no path is specified, the root path displays as `X:.` format; when explicitly specified, displays as a full uppercase path.
- `<OPTIONS>`: Can be repeated and mixed. Supports three forms listed in the table below: `--` (GNU-style, case-sensitive), `-` (short form, case-sensitive), and `/` (CMD-style, case-insensitive).

## Output Mode Description

`tree++` supports two output modes:

### Streaming Output (Default)

Scans, renders, and outputs simultaneously for real-time scrolling effect. Suitable for most interactive scenarios.

### Batch Mode

Explicitly enabled via `--batch` (`-b` / `/B`). Outputs only after complete scanning. The following features **require batch mode**:

- Structured output formats (JSON, YAML, TOML)
- `/DU` (cumulative directory size, requires complete tree calculation)
- `/T` (multi-threaded scanning)

## Detailed Option Descriptions

### `/?`: Show Help

**Function:** Displays complete option help information.

**Syntax:**

```powershell
treepp (--help | -h | /?)
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /?
tree++: A much better Windows tree command.

Usage:
  treepp [<PATH>] [<OPTIONS>...]

Options:
  --help, -h, /?              Show help information
  --version, -v, /V           Show version information
  --batch, -b, /B             Use batch processing mode
  --ascii, -a, /A             Draw the tree using ASCII characters
  --files, -f, /F             Show files
  --full-path, -p, /FP        Show full paths
  --human-readable, -H, /HR   Show file sizes in human-readable format
  --no-indent, -i, /NI        Do not display tree connector lines
  --reverse, -r, /R           Sort in reverse order
  --size, -s, /S              Show file size (bytes)
  --date, -d, /DT             Show last modified date
  --exclude, -I, /X <PATTERN> Exclude files matching the pattern
  --level, -L, /L <N>         Limit recursion depth
  --include, -m, /M <PATTERN> Show only files matching the pattern
  --disk-usage, -u, /DU       Show cumulative directory sizes (requires --batch)
  --report, -e, /RP           Show summary statistics at the end
  --prune, -P, /P             Prune empty directories
  --no-win-banner, -N, /NB    Do not show the Windows native tree banner/header
  --silent, -l, /SI           Silent mode (requires --output)
  --output, -o, /O <FILE>     Write output to a file (.txt, .json, .yml, .toml)
                              Note: JSON/YAML/TOML formats require --batch
  --thread, -t, /T <N>        Number of scanning threads (requires --batch, default: 8)
  --gitignore, -g, /G         Respect .gitignore

More info: https://github.com/Water-Run/treepp
```

### `/V`: Show Version

**Function:** Outputs current version information.

**Syntax:**

```powershell
treepp (--version | -v | /V)
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /V
tree++ version 0.1.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

### `/B`: Batch Mode

**Function:** Enables batch mode, outputting only after complete scanning. Some features (such as structured output, disk usage calculation, multi-threaded scanning) require this mode.

**Syntax:**

```powershell
treepp (--batch | -b | /B) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /B /F /DU
```

### `/A`: Draw Tree Using ASCII Characters

**Function:** Outputs using ASCII tree characters, compatible with `tree /A`.

**Syntax:**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
```

**Tree Symbol Comparison:**

| Mode    | Branch | Last Branch | Vertical | Indent   |
|---------|--------|-------------|----------|----------|
| Unicode | `├─`   | `└─`        | `│`      | 4 spaces |
| ASCII   | `+---` | `\---`      | `        | `        | 4 spaces |

**Example:**

```powershell
PS D:\Rust\tree++> treepp /A
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
\---src
```

### `/F`: Show Files

**Function:** Lists file entries in the directory tree.

**Syntax:**

```powershell
treepp (--files | -f | /F) [<PATH>]
```

**Example (combined with `/A`):**

```powershell
PS D:\Rust\tree++> treepp /A /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
|   Cargo.toml
|   LICENSE
|   OPTIONS-zh.md
|   OPTIONS.md
|   README-zh.md
|   README.md
|
\---src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

### `/FP`: Show Full Paths

**Function:** Displays all entries with absolute paths.

**Syntax:**

```powershell
treepp (--full-path | -p | /FP) [<PATH>]
```

**Example (combined with `/F`):**

```powershell
PS D:\Rust\tree++> treepp /F /FP
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
│   D:\Rust\tree++\Cargo.toml
│   D:\Rust\tree++\LICENSE
│   D:\Rust\tree++\OPTIONS-zh.md
│   D:\Rust\tree++\OPTIONS.md
│   D:\Rust\tree++\README-zh.md
│   D:\Rust\tree++\README.md
│
└─D:\Rust\tree++\src
        D:\Rust\tree++\src\cli.rs
        D:\Rust\tree++\src\config.rs
        D:\Rust\tree++\src\error.rs
        D:\Rust\tree++\src\main.rs
        D:\Rust\tree++\src\output.rs
        D:\Rust\tree++\src\render.rs
        D:\Rust\tree++\src\scan.rs
```

### `/HR`: Human-Readable File Sizes

**Function:** Converts file sizes to readable units such as B/KB/MB/GB/TB. Enabling this option automatically enables `/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**Example (`/HR /F`):**

```powershell
PS D:\Rust\tree++> treepp /HR /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml        982 B
│   LICENSE           1.0 KB
│   OPTIONS-zh.md     7.9 KB
│   OPTIONS.md        7.5 KB
│   README-zh.md      10.2 KB
│   README.md         9.1 KB
│
└─src
        cli.rs         6.0 KB
        config.rs      2.8 KB
        error.rs       1.9 KB
        main.rs        512 B
        output.rs      7.3 KB
        render.rs      5.2 KB
        scan.rs        8.8 KB
```

### `/S`: Show File Size (Bytes)

**Function:** Displays file size in bytes. Can be combined with `/HR` to convert to human-readable format.

**Syntax:**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /S /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml        982
│   LICENSE           1067
│   OPTIONS-zh.md     8120
│   OPTIONS.md        7644
│   README-zh.md      10420
│   README.md         9288
│
└─src
        cli.rs         6120
        config.rs      2840
        error.rs       1980
        main.rs        512
        output.rs      7440
        render.rs      5360
        scan.rs        9020
```

### `/NI`: No Tree Connector Lines

**Function:** Uses plain space indentation instead of tree symbols (2 spaces per level).

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NI
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
  Cargo.toml
  LICENSE
  OPTIONS-zh.md
  OPTIONS.md
  README-zh.md
  README.md
  src
    cli.rs
    config.rs
    error.rs
    main.rs
    output.rs
    render.rs
    scan.rs
```

### `/R`: Reverse Sort

**Function:** Reverses the current sort order.

**Syntax:**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /R
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   README.md
│   README-zh.md
│   OPTIONS.md
│   OPTIONS-zh.md
│   LICENSE
│   Cargo.toml
│
└─src
        scan.rs
        render.rs
        output.rs
        main.rs
        error.rs
        config.rs
        cli.rs
```

### `/DT`: Show Last Modified Date

**Function:** Appends the last modification time of files/directories after each entry, formatted as `YYYY-MM-DD HH:MM:SS` (local timezone).

**Syntax:**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /DT
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml        2025-12-16 10:02:11
│   LICENSE           2024-11-03 09:00:29
│   OPTIONS-zh.md     2025-12-17 14:20:16
│   OPTIONS.md        2025-12-17 14:18:05
│   README-zh.md      2025-12-18 09:12:40
│   README.md         2025-12-18 09:10:03
│
└─src
        cli.rs         2025-12-17 22:41:12
        config.rs      2025-12-17 22:35:09
        error.rs       2025-12-17 22:12:47
        main.rs        2025-12-17 20:30:00
        output.rs      2025-12-17 23:01:58
        render.rs      2025-12-17 22:58:47
        scan.rs        2025-12-17 23:05:58
```

### `/X`: Exclude Matches

**Function:** Ignores files or directories matching the pattern. Supports wildcards `*` and `?`. Can be specified multiple times to exclude multiple patterns.

**Syntax:**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**Example (exclude `*.md`):**

```powershell
PS D:\Rust\tree++> treepp /F /X *.md
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

**Example (exclude multiple patterns):**

```powershell
PS D:\Rust\tree++> treepp /F /X *.md /X LICENSE
```

### `/L`: Limit Recursion Depth

**Function:** Specifies maximum recursion level. `0` shows only the root directory itself, `1` shows the root and its direct children.

**Syntax:**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**Example (show only 1 level):**

```powershell
PS D:\Rust\tree++> treepp /F /L 1
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
```

### `/M`: Show Only Matches

**Function:** Retains only file entries matching the pattern (directories are always shown to maintain structure). Supports wildcards. Can be specified multiple times.

**Syntax:**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**Example (show only `*.rs`):**

```powershell
PS D:\Rust\tree++> treepp /F /M *.rs
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

### `/DU`: Show Cumulative Directory Size

**Function:** Calculates cumulative disk usage for each directory (recursively sums all child file sizes). Often used with `/HR`.

> **Note:** This option requires batch mode (`/B`) because it needs to scan the complete tree before calculating cumulative sizes.

**Syntax:**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /B /DU /HR
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
└─src        31.5 KB
```

### `/RP`: Show Summary Report

**Function:** Appends summary statistics at the end of output, including directory count, file count (if `/F` is enabled), and scan duration.

**Syntax:**

```powershell
treepp (--report | -e | /RP) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /RP
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs

1 directory, 13 files in 0.015s
```

### `/P`: Prune Empty Directories

**Function:** Hides directory nodes that don't contain any files (recursive check: directories containing only empty subdirectories are also considered empty).

**Syntax:**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /P /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

### `/NB`: No Windows Banner

**Function:** Omits the Windows native `tree` volume information and serial number output (first two lines).

**Syntax:**

```powershell
treepp (--no-win-banner | -N | /NB) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NB
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

> **Performance Tip:** The banner information is obtained by executing the native `tree` command in the `X:\__tree++__` directory. Enabling this option is recommended in performance-sensitive scenarios.

### `/SI`: Silent Mode

**Function:** Prevents writing results to standard output.

> **Restriction:** Must be used with `/O`, otherwise an error will be reported. Using silent mode alone is meaningless (no output is produced).

**Syntax:**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**Example (`/F /O tree.txt /SI`):**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.txt /SI
PS D:\Rust\tree++>
```

**Error Example (missing `/O`):**

```powershell
PS D:\Rust\tree++> treepp /SI
tree++: Config error: Option conflict: --silent and (no --output) cannot be used together (Silent mode requires an output file; otherwise no output will be produced.)
```

### `/O`: Output to File

**Function:** Persists results to a file. Supported formats are determined by file extension. By default, still outputs to console; can be combined with `/SI` for silence.

**Syntax:**

```powershell
treepp (--output | -o | /O) <FILE> [<PATH>]
```

**Supported Extensions:**

| Extension      | Format     | Requires `/B` |
|----------------|------------|---------------|
| `.txt`         | Plain text | No            |
| `.json`        | JSON       | Yes           |
| `.yml` `.yaml` | YAML       | Yes           |
| `.toml`        | TOML       | Yes           |

> **Note:** Structured output formats (JSON/YAML/TOML) require batch mode (`/B`).

**Example (TXT format, no `/B` needed):**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.txt
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs

output: D:\Rust\tree++\tree.txt
```

**Example (JSON format, requires `/B`):**

```powershell
PS D:\Rust\tree++> treepp /B /F /O tree.json
```

### `/T`: Scanning Thread Count

**Function:** Specifies the number of scanning threads. Value must be a positive integer.

> **Restriction:** This option requires batch mode (`/B`).

**Syntax:**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**Default Value:** 8

**Example:**

```powershell
PS D:\Rust\tree++> treepp /B /F /T 16
```

**Error Example (missing `/B`):**

```powershell
PS D:\Rust\tree++> treepp /F /T 16
tree++: CLI error: Option conflict: --thread and (no --batch) cannot be used together
```

### `/G`: Honor `.gitignore`

**Function:** Parses `.gitignore` files in each directory level, automatically ignoring matching entries. Supports rule chain inheritance: subdirectories inherit parent directory rules while applying their own rules.

**Syntax:**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /G
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs
```

## Option Restriction Summary

| Option | Restriction Description                                                                         |
|--------|-------------------------------------------------------------------------------------------------|
| `/SI`  | Must be used with `/O`                                                                          |
| `/T`   | Value must be a positive integer (≥1), and requires `/B`                                        |
| `/L`   | Value must be a non-negative integer (≥0)                                                       |
| `/DU`  | Requires `/B`                                                                                   |
| `/O`   | Extension must be `.txt`, `.json`, `.yml`, `.yaml`, or `.toml`; structured formats require `/B` |

## Exit Codes

| Exit Code | Meaning        |
|-----------|----------------|
| 0         | Success        |
| 1         | Argument error |
| 2         | Scan error     |
| 3         | Output error   |
