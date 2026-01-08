# `tree++`: Complete Options Documentation and Examples

This document describes all supported options and usage examples for [tree++](https://github.com/Water-Run/treepp).

## Sample Directory

All command examples are based on this sample directory structure:

```powershell
PS D:\Rust\tree++> treepp /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

> `treepp /F` behaves identically to the native Windows `tree /F`: displays volume header and tree structure. Running `treepp` alone maintains the original semantics of showing only directory structure.

## General Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`: Optional, defaults to current directory. When no path is specified, the root is displayed as `X:.`; when explicitly specified, it shows the full uppercase path.
- `<OPTIONS>`: Can be repeated and mixed. Supports three forms listed in the table below: `--` (GNU-style, case-sensitive), `-` (short options, case-sensitive), and `/` (CMD-style, case-insensitive).

## Output Mode Explanation

### Streaming Output

`tree++` uses **streaming output** mode by default, rendering and displaying results as it scans for real-time scrolling effect.

The following conditions will **fall back to batch mode** (complete scan before output):

- Non-TXT output format (e.g., JSON, YAML, TOML)
- `/DU` enabled (cumulative directory size requires full tree calculation)
- Output file specified (`/O`)
- Silent mode enabled (`/SI`)

## Detailed Option Descriptions

### `/?`: Show Help

**Function:** Display complete option help information.

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
  --disk-usage, -u, /DU       Show cumulative directory sizes
  --ignore-case, -c, /IC      Case-insensitive matching
  --report, -e, /RP           Show summary statistics at the end
  --prune, -P, /P             Prune empty directories
  --sort, -S, /SO <KEY>       Set sort mode (name, size, mtime, ctime)
  --no-win-banner, -N, /NB    Do not show the Windows native tree banner/header
  --silent, -l, /SI           Silent mode (use with --output)
  --output, -o, /O <FILE>     Write output to a file (.txt, .json, .yml, .toml)
  --thread, -t, /T <N>        Number of scanning threads (default: 8)
  --gitignore, -g, /G         Respect .gitignore
  --quote, -q, /Q             Wrap file names in double quotes
  --dirs-first, -D, /DF       List directories first

More info: https://github.com/Water-Run/treepp
```

### `/V`: Show Version

**Function:** Display current version information.

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

### `/A`: Draw Tree with ASCII Characters

**Function:** Output using ASCII tree characters, compatible with `tree /A`.

**Syntax:**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
```

**Tree Character Comparison:**

| Mode    | Branch | Last Branch | Vertical | Indent   |
|---------|--------|-------------|----------|----------|
| Unicode | `├─`   | `└─`        | `│   `   | 4 spaces |
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

**Function:** List file entries in the directory tree.

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

**Function:** Display all entries with absolute paths.

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

**Function:** Convert file sizes to readable units like B/KB/MB/GB/TB. Enabling this option automatically enables `/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**Example (`/HR /F`):**

```powershell
PS D:\Rust\tree++> treepp /HR /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Display file size in bytes. Can be combined with `/HR` for human-readable format.

**Syntax:**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /S /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Use plain space indentation instead of tree symbols (2 spaces per level).

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NI
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

### `/R`: Reverse Sort Order

**Function:** Reverse the current sort order. Can be combined with `/SO`.

**Syntax:**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /R
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Append last modified time to each entry in `YYYY-MM-DD HH:MM:SS` format (local timezone).

**Syntax:**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /DT
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

### `/X`: Exclude Matching Items

**Function:** Ignore files or directories matching the pattern. Supports wildcards `*` and `?`. Can be specified multiple times.

**Syntax:**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**Example (exclude `*.md`):**

```powershell
PS D:\Rust\tree++> treepp /F /X *.md
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Specify maximum recursion level. `0` shows only the root directory itself, `1` shows root and its direct children.

**Syntax:**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**Example (show only 1 level):**

```powershell
PS D:\Rust\tree++> treepp /F /L 1
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
```

### `/M`: Show Only Matching Items

**Function:** Keep only files matching the pattern (directories always shown to maintain structure). Supports wildcards. Can be specified multiple times.

**Syntax:**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**Example (show only `*.rs`):**

```powershell
PS D:\Rust\tree++> treepp /F /M *.rs
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Calculate cumulative disk usage for each directory (recursively summing all child file sizes). Often used with `/HR`.

> **Note:** Enabling this option disables streaming output as it requires a complete tree scan to calculate cumulative sizes.

**Syntax:**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /DU /HR
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
└─src        31.5 KB
```

### `/IC`: Ignore Case in Matching

**Function:** Make `/M`, `/X` and other matching options case-insensitive.

**Syntax:**

```powershell
treepp (--ignore-case | -c | /IC) [<PATH>]
```

**Example (`/F /M *.MD /IC`):**

```powershell
PS D:\Rust\tree++> treepp /F /M *.MD /IC
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
│
└─src
```

### `/RP`: Show Summary Statistics

**Function:** Append summary statistics at the end of output, including directory count, file count (if `/F` enabled), and scan time.

**Syntax:**

```powershell
treepp (--report | -e | /RP) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /RP
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

**Function:** Hide directory nodes containing no files (recursive: directories containing only empty subdirectories are also considered empty).

**Syntax:**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /P /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

### `/SO`: Specify Sort Method

**Function:** Sort by specified field (case-insensitive). Can be combined with `/R` for descending order.

**Syntax:**

```powershell
treepp (--sort | -S | /SO) <KEY> [<PATH>]
```

**Available Sort Fields:**

| Field   | Description                                                    |
|---------|----------------------------------------------------------------|
| `name`  | Alphabetical ascending by filename (default, case-insensitive) |
| `size`  | Ascending by file size (directories use cumulative size or 0)  |
| `mtime` | Ascending by last modified time                                |
| `ctime` | Ascending by creation time                                     |

**Example (`/F /SO size /R`, descending by size):**

```powershell
PS D:\Rust\tree++> treepp /F /SO size /R
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
│   README-zh.md
│   README.md
│   OPTIONS-zh.md
│   OPTIONS.md
│   LICENSE
│   Cargo.toml
│
└─src
        scan.rs
        output.rs
        cli.rs
        render.rs
        config.rs
        error.rs
        main.rs
```

### `/NB`: No Windows Banner

**Function:** Omit the Windows native `tree` volume information and serial number output (first two lines).

**Syntax:**

```powershell
treepp (--no-win-banner | -N | /NB) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NB
D:\RUST\TREE++
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

> **Performance Note:** Banner information is obtained by executing the native `tree` command in the `X:\__tree++__` directory. Enabling this option is recommended for performance-sensitive scenarios.

### `/SI`: Silent Terminal Output

**Function:** Suppress output to standard output.

> **Restriction:** Must be used with `/O`, otherwise an error will occur. Using silent mode alone has no meaning (no output is produced).

**Syntax:**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**Example (`/F /O tree.json /SI`):**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json /SI
PS D:\Rust\tree++>
```

**Error Example (missing `/O`):**

```powershell
PS D:\Rust\tree++> treepp /SI
tree++: Config error: Option conflict: --silent and (no --output) cannot be used together (Silent mode requires an output file; otherwise no output will be produced.)
```

### `/O`: Output to File

**Function:** Persist results to a file. Supported formats are determined by file extension. By default, output is still displayed in the console; combine with `/SI` for silent mode.

**Syntax:**

```powershell
treepp (--output | -o | /O) <FILE> [<PATH>]
```

**Supported Extensions:**

| Extension      | Format     |
|----------------|------------|
| `.txt`         | Plain text |
| `.json`        | JSON       |
| `.yml` `.yaml` | YAML       |
| `.toml`        | TOML       |

> **Note:** Specifying an output file disables streaming output mode.

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

output: D:\Rust\tree++\tree.json
```

### `/T`: Scan Thread Count

**Function:** Specify the number of scanning threads. Value must be a positive integer.

**Syntax:**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**Default:** 8

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /T 16
```

### `/G`: Respect `.gitignore`

**Function:** Parse `.gitignore` files in each directory level and automatically ignore matching entries. Supports rule chain inheritance: subdirectories inherit parent rules while applying their own.

**Syntax:**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /G
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
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

### `/Q`: Quote File Names

**Function:** Wrap all file and directory names in double quotes in the output.

**Syntax:**

```powershell
treepp (--quote | -q | /Q) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /Q
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
│   "Cargo.toml"
│   "LICENSE"
│   "OPTIONS-zh.md"
│   "OPTIONS.md"
│   "README-zh.md"
│   "README.md"
│
└─"src"
        "cli.rs"
        "config.rs"
        "error.rs"
        "main.rs"
        "output.rs"
        "render.rs"
        "scan.rs"
```

### `/DF`: Directories First

**Function:** In sorted results, directories always appear before files.

**Syntax:**

```powershell
treepp (--dirs-first | -D | /DF) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /DF
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\RUST\TREE++
├─src
│       cli.rs
│       config.rs
│       error.rs
│       main.rs
│       output.rs
│       render.rs
│       scan.rs
│
│   Cargo.toml
│   LICENSE
│   OPTIONS-zh.md
│   OPTIONS.md
│   README-zh.md
│   README.md
```

## Option Restrictions Summary

| Option | Restriction                                                    |
|--------|----------------------------------------------------------------|
| `/SI`  | Must be used with `/O`                                         |
| `/T`   | Value must be a positive integer (≥1)                          |
| `/L`   | Value must be a non-negative integer (≥0)                      |
| `/O`   | Extension must be `.txt`, `.json`, `.yml`, `.yaml`, or `.toml` |

## Exit Codes

| Code | Meaning        |
|------|----------------|
| 0    | Success        |
| 1    | Argument error |
| 2    | Scan error     |
| 3    | Output error   |