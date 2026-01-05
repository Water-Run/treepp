# `tree++`: Complete Parameter Documentation and Examples

This document briefly describes all parameters and usage examples supported by [tree++](https://github.com/Water-Run/treepp).

## Mock Directory

Command example outputs are based on this mock directory:

```powershell
PS D:\Rust\tree++> treepp /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

> `treepp /F` behaves exactly like Windows native `tree /F`: displays volume header and tree structure. Running `treepp` directly maintains the original semantics of showing only directory structure.

## Global Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`: Optional, defaults to current directory.
- `<OPTIONS>`: Repeatable and mixable. Supports `--` (GNU), `-` (short), and `/` (CMD, case-insensitive) forms listed in the table below.

## Detailed Command Descriptions

### `/?`: Show Help

**Function:** Display complete parameter help information.

**Syntax:**

```powershell
treepp (--help | -h | /?) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /?
tree++ - a better tree command for Windows
Usage:
  treepp [path] [options]
Options:
  /H, /?, -h, --help        Show help information
  /V, -v, --version         Show version information
  ...
```

### `/V`: Show Version

**Function:** Output current version information.

**Syntax:**

```powershell
treepp (--version | -v | /V)
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /V
tree++ version 0.1.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

### `/A`: Draw Tree Using ASCII Characters

**Function:** Output with ASCII tree characters, compatible with `tree /A`.

**Syntax:**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /A
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
D:\Rust\tree++
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
        main.rs
        error.rs
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
D:\Rust\tree++
│  D:\Rust\tree++\Cargo.toml
│  D:\Rust\tree++\LICENSE
│  D:\Rust\tree++\OPTIONS-zh.md
│  D:\Rust\tree++\OPTIONS.md
│  D:\Rust\tree++\README-zh.md
│  D:\Rust\tree++\README.md
│
└─D:\Rust\tree++\src
        D:\Rust\tree++\src\cli.rs
        D:\Rust\tree++\src\config.rs
        D:\Rust\tree++\src\main.rs
        D:\Rust\tree++\src\output.rs
        D:\Rust\tree++\src\render.rs
        D:\Rust\tree++\src\scan.rs
```

### `/HR`: Human-Readable File Sizes

**Function:** Convert file sizes to readable units like B/KB/MB, often used with `/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**Example (`/S /HR /F`):**

```powershell
PS D:\Rust\tree++> treepp /S /HR /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml        982 B
│  LICENSE           1.0 KB
│  OPTIONS-zh.md     7.9 KB
│  OPTIONS.md        7.5 KB
│  README-zh.md      10.2 KB
│  README.md         9.1 KB
│
└─src
        cli.rs         6.0 KB
        config.rs      2.8 KB
        main.rs
        error.rs        1.9 KB
        output.rs      7.3 KB
        render.rs      5.2 KB
        scan.rs        8.8 KB
```

### `/S`: Show File Sizes (Bytes)

**Function:** Display file sizes in bytes, can be used with `/HR`.

**Syntax:**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /S /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml        982
│  LICENSE           1067
│  OPTIONS-zh.md     8120
│  OPTIONS.md        7644
│  README-zh.md     10420
│  README.md         9288
│
└─src
        cli.rs         6120
        config.rs      2840
        main.rs
        error.rs        1980
        output.rs      7440
        render.rs      5360
        scan.rs        9020
```

### `/NI`: No Tree Connector Lines

**Function:** Use indentation instead of tree symbols.

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NI
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
  Cargo.toml
  LICENSE
  OPTIONS-zh.md
  OPTIONS.md
  README-zh.md
  README.md

  src
    cli.rs
    config.rs
    main.rs
    output.rs
    render.rs
    scan.rs
```

### `/R`: Reverse Sort

**Function:** Reverse the current sort order, can be combined with `/SO`.

**Syntax:**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /R
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  README.md
│  README-zh.md
│  OPTIONS.md
│  OPTIONS-zh.md
│  LICENSE
│  Cargo.toml
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

**Function:** Append the last modified time of files/directories.

**Syntax:**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /DT
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml        2025-12-16 10:02:11
│  LICENSE           2024-11-03 09:00:29
│  OPTIONS-zh.md     2025-12-17 14:20:16
│  OPTIONS.md        2025-12-17 14:18:05
│  README-zh.md      2025-12-18 09:12:40
│  README.md         2025-12-18 09:10:03
│
└─src
        cli.rs         2025-12-17 22:41:12
        config.rs      2025-12-17 22:35:09
        main.rs
        error.rs        2025-12-17 22:12:47
        output.rs      2025-12-17 23:01:58
        render.rs      2025-12-17 22:58:47
        scan.rs        2025-12-17 23:05:58
```

### `/X`: Exclude Matching Items

**Function:** Ignore files or directories matching the pattern.

**Syntax:**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**Example (exclude `*.md`):**

```powershell
PS D:\Rust\tree++> treepp /F /X *.md
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml
│  LICENSE
│
└─src
        cli.rs
        config.rs
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

### `/L`: Limit Recursion Depth

**Function:** Specify maximum recursion level.

**Syntax:**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**Example (show only 1 level):**

```powershell
PS D:\Rust\tree++> treepp /F /L 1
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
```

### `/M`: Show Only Matching Items

**Function:** Keep only entries matching the pattern.

**Syntax:**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**Example (show only `*.rs`):**

```powershell
PS D:\Rust\tree++> treepp /F /M *.rs
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│
└─src
        cli.rs
        config.rs
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

### `/DU`: Show Cumulative Directory Size

**Function:** Calculate cumulative disk usage for each directory, works with `/HR`.

**Syntax:**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /DU /HR
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
src             31.5 KB
```

### `/IC`: Ignore Case When Matching

**Function:** Make `/M`, `/X` and other matching commands case-insensitive.

**Syntax:**

```powershell
treepp (--ignore-case | -c | /IC) [<PATH>]
```

**Example (`/F /M *.MD /IC`):**

```powershell
PS D:\Rust\tree++> treepp /F /M *.MD /IC
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

### `/RP`: Show Trailing Statistics

**Function:** Display summary statistics at the end.

**Syntax:**

```powershell
treepp (--report | -e | /RP) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /RP
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs

1 directory, 12 files in 0.123s
```

### `/P`: Prune Empty Directories

**Function:** Hide directory nodes that contain no content.

**Syntax:**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /P /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

### `/SO`: Specify Sort Method

**Function:** Sort by fields like `name`, `size`, `mtime`, etc., can be combined with `/R`.

**Syntax:**

```powershell
treepp (--sort | -S | /SO) <KEY> [<PATH>]
```

**Example (`/F /SO name`):**

```powershell
PS D:\Rust\tree++> treepp /F /SO name
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

*Available sort fields and descriptions:*

| Field   | Description                      |
|---------|----------------------------------|
| `name`  | Sort alphabetically by filename  |
| `size`  | Sort by file size ascending      |
| `mtime` | Sort by modification time        |
| `ctime` | Sort by creation time            |

### `/NB`: Hide Windows-native banner output

**Purpose:**
Omit the Windows-native `tree` banner output.

**Syntax:**

```powershell
treepp (--no-win-banner | -N | /NB) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NB
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

> Recommended for performance-sensitive scenarios: in `tree++`, this banner output is obtained by invoking the native `tree` against the target `C:\__treepp__`.


### `/SI`: Silent Terminal Output

**Function:** Suppress standard output, typically used with `/O` to silently write to file.

**Syntax:**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**Example (`/F /O tree.json /SI`):**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json /SI
PS D:\Rust\tree++>
```

### `/O`: Output to File

**Function:** Persist results to `.txt` / `.json` / `.yml` / `.toml` files. Console output remains by default, can be silenced with `/SI`.

**Syntax:**

```powershell
treepp (--output | -o | /O) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs

output: D:\Rust\tree++\tree.json
```

### `/T`: Scan Thread Count

**Function:** Specify number of scan threads, default is 8.

**Syntax:**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /T 32
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs
```

### `/G`: Honor `.gitignore`

**Function:** Parse `.gitignore` files in each directory level and automatically ignore matching entries.

**Syntax:**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /G
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
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
        main.rs
        error.rs
        output.rs
        render.rs
        scan.rs

.gitignore rules applied
```
