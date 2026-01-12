# `tree++`: Complete Parameter Documentation and Examples

This document provides a comprehensive overview of all parameters and usage examples for [tree++](https://github.com/Water-Run/treepp).

## Sample Directory

Command examples are based on this sample directory structure:

```powershell
PS D:\Data\Rust\tree++> treepp /f
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
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

> `treepp /F` behaves identically to Windows native `tree /F` (diff-level): displays volume header information and tree structure. Running `treepp` alone maintains the original behavior of showing only directory structure.

## General Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`: Optional, defaults to current directory. When no path is specified, the root is displayed as `X:.` format; when explicitly specified, it shows the full uppercase path.
- `<OPTIONS>`: Can be repeated and mixed. Supports three forms listed in the table below: `--` (GNU-style, case-sensitive), `-` (short parameters, case-sensitive), and `/` (CMD-style, case-insensitive).

## Output Modes

`tree++` supports two output modes:

### Streaming Output (Default)

Scans, renders, and outputs simultaneously, providing real-time scrolling. Suitable for most interactive scenarios.

### Batch Processing Mode

Explicitly enabled via `--batch` (`-b` / `/B`). Performs complete scan before unified output. The following features **require batch processing mode**:

- Structured output formats (JSON, YAML, TOML)
- `/DU` (cumulative directory size, requires full tree calculation)
- `/T` (multi-threaded scanning)

## Detailed Parameter Documentation

### `/?`: Show Help

**Function:** Displays complete parameter help information.

**Syntax:**

```powershell
treepp (--help | -h | /?)
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /?
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
PS D:\Data\Rust\tree++> treepp /v
tree++ version 0.1.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

### `/B`: Batch Processing Mode

**Function:** Enables batch processing mode, performing complete scan before unified output. Some features (such as structured output, disk usage calculation, multi-threaded scanning) require this mode.

**Syntax:**

```powershell
treepp (--batch | -b | /B) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /b /f /du
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        1698
│  Cargo.lock        19029
│  Cargo.toml        1028
│  LICENSE        35821
│  OPTIONS-zh.md        19048
│  OPTIONS.md        18812
│  README-zh.md        4487
│  README.md        4915
│
└─src        387614
        cli.rs        68292
        config.rs        41695
        error.rs        28022
        main.rs        11041
        output.rs        25693
        render.rs        118425
        scan.rs        94446
```

### `/A`: Draw Tree with ASCII Characters

**Function:** Outputs tree using ASCII characters, compatible with `tree /A`.

**Syntax:**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /a
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
PS D:\Data\Rust\tree++> treepp /a /f
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
|   .gitignore
|   Cargo.lock
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
PS D:\Data\Rust\tree++> treepp /f /fp
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  D:\Data\Rust\tree++\.gitignore
│  D:\Data\Rust\tree++\Cargo.lock
│  D:\Data\Rust\tree++\Cargo.toml
│  D:\Data\Rust\tree++\LICENSE
│  D:\Data\Rust\tree++\OPTIONS-zh.md
│  D:\Data\Rust\tree++\OPTIONS.md
│  D:\Data\Rust\tree++\README-zh.md
│  D:\Data\Rust\tree++\README.md
│
└─D:\Data\Rust\tree++\src
        D:\Data\Rust\tree++\src\cli.rs
        D:\Data\Rust\tree++\src\config.rs
        D:\Data\Rust\tree++\src\error.rs
        D:\Data\Rust\tree++\src\main.rs
        D:\Data\Rust\tree++\src\output.rs
        D:\Data\Rust\tree++\src\render.rs
        D:\Data\Rust\tree++\src\scan.rs
```

### `/HR`: Human-Readable File Sizes

**Function:** Converts file sizes to readable units like B/KB/MB/GB/TB. Enabling this option automatically enables `/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**Example (`/HR /F`):**

```powershell
PS D:\Data\Rust\tree++> treepp /hr /f
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        1.7 KB
│  Cargo.lock        18.6 KB
│  Cargo.toml        1.0 KB
│  LICENSE        35.0 KB
│  OPTIONS-zh.md        18.6 KB
│  OPTIONS.md        18.4 KB
│  README-zh.md        4.4 KB
│  README.md        4.8 KB
│
└─src
        cli.rs        66.7 KB
        config.rs        40.7 KB
        error.rs        27.4 KB
        main.rs        10.8 KB
        output.rs        25.1 KB
        render.rs        115.6 KB
        scan.rs        92.2 KB
```

### `/S`: Show File Size (Bytes)

**Function:** Displays file size in bytes, can be combined with `/HR` for human-readable format.

**Syntax:**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**Example:**

```powershell
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        1698
│  Cargo.lock        19029
│  Cargo.toml        1028
│  LICENSE        35821
│  OPTIONS-zh.md        19048
│  OPTIONS.md        18812
│  README-zh.md        4487
│  README.md        4915
│
└─src
        cli.rs        68292
        config.rs        41695
        error.rs        28022
        main.rs        11041
        output.rs        25693
        render.rs        118425
        scan.rs        94446
```

### `/NI`: No Tree Connector Lines

**Function:** Uses plain space indentation instead of tree symbols (2 spaces per level).

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /ni
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
.gitignore
Cargo.lock
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

**Function:** Reverses the current sort order.

**Syntax:**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /r
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  README.md
│  README-zh.md
│  OPTIONS.md
│  OPTIONS-zh.md
│  LICENSE
│  Cargo.toml
│  Cargo.lock
│  .gitignore
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

**Function:** Appends last modification time for files/directories in `YYYY-MM-DD HH:MM:SS` format (local timezone).

**Syntax:**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /dt
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        2026-01-09 14:33:52
│  Cargo.lock        2026-01-06 15:23:37
│  Cargo.toml        2026-01-06 15:23:31
│  LICENSE        2025-12-09 15:04:28
│  OPTIONS-zh.md        2026-01-12 16:42:14
│  OPTIONS.md        2026-01-09 14:45:31
│  README-zh.md        2026-01-12 14:27:31
│  README.md        2026-01-12 14:28:12
│
└─src        2026-01-12 16:37:36
        cli.rs        2026-01-08 14:24:42
        config.rs        2026-01-12 09:34:42
        error.rs        2026-01-08 14:19:05
        main.rs        2026-01-12 16:28:17
        output.rs        2026-01-08 14:22:43
        render.rs        2026-01-12 16:24:44
        scan.rs        2026-01-12 16:37:36
```

### `/X`: Exclude Pattern

**Function:** Ignores files or directories matching the pattern. Supports wildcards `*` and `?`. Can be specified multiple times to exclude multiple patterns.

**Syntax:**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**Example (exclude `*.md`):**

```powershell
PS D:\Data\Rust\tree++> treepp /f /x *.md
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
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
PS D:\Data\Rust\tree++> treepp /f /x *.md /x LICENSE
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
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

### `/L`: Limit Recursion Depth

**Function:** Specifies maximum recursion level. `0` shows only the root directory itself, `1` shows root and its direct children.

**Syntax:**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**Example (show only 1 level):**

```powershell
PS D:\Data\Rust\tree++> treepp /f /l 1
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
```

### `/M`: Include Only Matching Files

**Function:** Retains only file entries matching the pattern (directories always shown to maintain structure). Supports wildcards. Can be specified multiple times.

**Syntax:**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**Example (show only `*.rs`):**

```powershell
PS D:\Data\Rust\tree++> treepp /f /m *rs
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
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

**Function:** Calculates cumulative disk usage for each directory (recursively sums all child file sizes). Often used with `/HR`. Enabling this option automatically enables `/S`.

> **Note:** This option requires batch processing mode (`/B`) because it needs a complete tree scan to calculate cumulative sizes.

**Syntax:**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /b /du /hr
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
└─src        378.5 KB
```

### `/RP`: Show Summary Report

**Function:** Appends statistical summary at the end of output, including directory count, file count (if `/F` enabled), and scan duration.

**Syntax:**

```powershell
treepp (--report | -e | /RP) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /rp
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs

1 directory, 15 files in 0.002s
```

### `/P`: Prune Empty Directories

**Function:** Hides directory nodes that don't contain any files (recursive check: directories containing only empty subdirectories are also considered empty).

**Syntax:**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**Example:**

Assuming an empty directory `empty_dir` exists, using `/P` will not display it:

```powershell
PS D:\Data\Rust\tree++> treepp /p /f
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
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

**Function:** Omits Windows native `tree` volume information and serial number output (first two lines).

**Syntax:**

```powershell
treepp (--no-win-banner | -N | /NB) [<PATH>]
```

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /nb
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
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

> **Performance Tip:** Banner information is obtained by executing the native `tree` command in the `X:\__tree++__` directory. Enabling this option is recommended for performance-sensitive scenarios.

### `/SI`: Silent Terminal Output

**Function:** Prevents writing results to standard output.

> **Restriction:** Must be used with `/O`, otherwise an error will occur. Using silent mode alone is meaningless (produces no output).

**Syntax:**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**Example (`/F /O tree.txt /SI`):**

```powershell
PS D:\Data\Rust\tree++> treepp /f /o tree.txt /si
```

### `/O`: Output to File

**Function:** Persists results to a file. Supported formats are determined by file extension. By default, still outputs to console; use `/SI` for silent mode.

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

> **Note:** Structured output formats (JSON/YAML/TOML) require batch processing mode (`/B`).

**Example (TXT format, no `/B` needed):**

```powershell
PS D:\Data\Rust\tree++> treepp /f /o tree.txt
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│  tree.txt
│
└─src
        cli.rs
        config.rs
        error.rs
        main.rs
        output.rs
        render.rs
        scan.rs


Output written to: tree.txt
```

**Example (JSON format, requires `/B`):**

```powershell
PS D:\Data\Rust\tree++> treepp /b /f /o tree.json
{
  ".gitignore": {},
  "Cargo.lock": {},
  "Cargo.toml": {},
  "LICENSE": {},
  "OPTIONS-zh.md": {},
  "OPTIONS.md": {},
  "README-zh.md": {},
  "README.md": {},
  "src": {
    "cli.rs": {},
    "config.rs": {},
    "error.rs": {},
    "main.rs": {},
    "output.rs": {},
    "render.rs": {},
    "scan.rs": {}
  },
  "tree.txt": {}
}
output: tree.json
```

### `/T`: Number of Scan Threads

**Function:** Specifies the number of scanning threads. Value must be a positive integer.

> **Restriction:** This option requires batch processing mode (`/B`).

**Syntax:**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**Default Value:** 8

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /b /f /t 16
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│  tree.json
│  tree.txt
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

### `/G`: Respect `.gitignore`

**Function:** Parses `.gitignore` files in each directory level and automatically ignores matching entries. Supports rule chain inheritance: subdirectories inherit parent directory rules while applying their own rules.

**Syntax:**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**Example:**

Assuming `.gitignore` contains `target/` and `*.log`, using `/G` will ignore these entries:

```powershell
PS D:\Data\Rust\tree++> treepp /f /g
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.lock
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│  tree.json
│  tree.txt
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

## Parameter Restrictions Summary

| Parameter | Restriction Description                                                                         |
|-----------|-------------------------------------------------------------------------------------------------|
| `/SI`     | Must be used with `/O`                                                                          |
| `/T`      | Value must be a positive integer (≥1) and requires `/B`                                         |
| `/L`      | Value must be a non-negative integer (≥0)                                                       |
| `/DU`     | Requires `/B`                                                                                   |
| `/O`      | Extension must be `.txt`, `.json`, `.yml`, `.yaml`, or `.toml`; structured formats require `/B` |

## Exit Codes

| Exit Code | Meaning         |
|-----------|-----------------|
| 0         | Success         |
| 1         | Parameter error |
| 2         | Scan error      |
| 3         | Output error    |