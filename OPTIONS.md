# `tree++`: Complete Parameter Documentation and Examples

This document provides a comprehensive overview of all parameters supported by [tree++](https://github.com/Water-Run/treepp) with usage examples.

## Mock Directory

The example outputs are based on this mock directory structure:

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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

> `treepp /F` behaves identically to the native Windows `tree /F` command: displaying volume header information, tree structure, and summary statistics. Running `treepp` alone maintains the original behavior of showing only the directory structure.

## Global Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`: Optional, defaults to current directory.
- `<OPTIONS>`: Can be repeated and mixed. Supports three formats listed in the table below: `--` (GNU style), `-` (short form), and `/` (CMD style, case-insensitive).

## Detailed Parameter Documentation

### `/?`: Display Help

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
  -h, --help        Show help information
  -v, --version     Show version information
  ...
```

### `/V`: Display Version

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

1 directories
```

### `/F`: Display Files

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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

### `/FP`: Display Full Paths

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

1 directories, 12 files
```

### `/HR`: Human-Readable File Sizes

**Function:** Convert file sizes to readable units like B/KB/MB, commonly used with `/S`.

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
        main.rs        1.9 KB
        output.rs      7.3 KB
        render.rs      5.2 KB
        scan.rs        8.8 KB

1 directories, 12 files
```

### `/S`: Display File Sizes (Bytes)

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
        main.rs        1980
        output.rs      7440
        render.rs      5360
        scan.rs        9020

1 directories, 12 files
```

### `/NI`: No Tree Connector Lines

**Function:** Replace tree symbols with indentation.

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

1 directories, 12 files
```

### `/R`: Reverse Sort Order

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
        config.rs
        cli.rs

1 directories, 12 files
```

### `/DT`: Display Last Modified Date

**Function:** Append the last modified time after each file/directory entry.

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
        main.rs        2025-12-17 22:12:47
        output.rs      2025-12-17 23:01:58
        render.rs      2025-12-17 22:58:47
        scan.rs        2025-12-17 23:05:58

1 directories, 12 files
```

### `/X`: Exclude Matching Items

**Function:** Ignore files or directories matching the pattern.

**Syntax:**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**Example (excluding `*.md`):**

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
        output.rs
        render.rs
        scan.rs

1 directories, 8 files
```

### `/L`: Limit Recursion Depth

**Function:** Specify maximum recursion levels.

**Syntax:**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**Example (showing only 1 level):**

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

1 directories, 6 files
```

### `/M`: Show Only Matching Items

**Function:** Keep only entries matching the pattern.

**Syntax:**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**Example (showing only `*.rs`):**

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
        output.rs
        render.rs
        scan.rs

1 directories, 6 files
```

### `/Q`: Quote File Names

**Function:** Wrap paths in double quotes for easier copying or script processing.

**Syntax:**

```powershell
treepp (--quote | -q | /Q) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /Q
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  "Cargo.toml"
│  "LICENSE"
│  "OPTIONS-zh.md"
│  "OPTIONS.md"
│  "README-zh.md"
│  "README.md"
│
└─"src"
        "cli.rs"
        "config.rs"
        "main.rs"
        "output.rs"
        "render.rs"
        "scan.rs"

1 directories, 12 files
```

### `/DF`: Directories First

**Function:** List directories before files in output.

**Syntax:**

```powershell
treepp (--dirs-first | -D | /DF) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /DF
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
└─src
        cli.rs
        config.rs
        main.rs
        output.rs
        render.rs
        scan.rs
│
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md

1 directories, 12 files
```

### `/DU`: Display Cumulative Directory Size

**Function:** Calculate cumulative disk usage for each directory, can be combined with `/HR`.

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

### `/IC`: Case-Insensitive Matching

**Function:** Make `/M`, `/X` and other matching commands ignore case.

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

0 directories, 4 files
```

### `/NR`: No Summary Report

**Function:** Omit the "X directories, Y files" summary.

**Syntax:**

```powershell
treepp (--no-report | -n | /NR) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NR
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
        output.rs
        render.rs
        scan.rs
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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

### `/SO`: Specify Sort Method

**Function:** Sort by fields like `name`, `size`, `mtime`, can be combined with `/R`.

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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

*Available sort fields:*

| Field   | Description                       |
|---------|-----------------------------------|
| `name`  | Sort by filename alphabetically   |
| `size`  | Sort by file size ascending       |
| `mtime` | Sort by modification time ascending |
| `ctime` | Sort by creation time ascending   |

### `/NH`: No Volume Information and Header

**Function:** Omit volume name, serial number, and other header content.

**Syntax:**

```powershell
treepp (--no-header | -N | /NH) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /F /NH
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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

### `/SI`: Silent Terminal Output

**Function:** Suppress output to stdout, typically used with `/O` to silently write to file.

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

**Function:** Persist results to `.txt` / `.json` / `.yml` / `.toml` files. Default still outputs to console, can be combined with `/SI` for silent operation.

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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files

output: D:\Rust\tree++\tree.json
```

### `/T`: Scan Thread Count

**Function:** Specify number of scanning threads, default is 24.

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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

### `/MFT`: Use MFT (Administrator Mode)

**Function:** Explicitly enable NTFS MFT scanning under administrator privileges, bypassing regular directory traversal for significantly improved performance on large directories. Requires administrator privileges.

> Recommend using with [Sudo For Windows](https://learn.microsoft.com/en-us/windows/advanced-settings/sudo/)

**Syntax:**

```powershell
sudo treepp [<PATH>] (--mft | -M | /MFT) [<OPTIONS>...]
```

**Example:**

```powershell
PS D:\Rust\tree++> sudo treepp /F /MFT
[MFT] enabled: scanning via NTFS Master File Table
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
        output.rs
        render.rs
        scan.rs

1 directories, 12 files
```

*Note that in MFT mode, the following commands cannot be used, otherwise an exception will be thrown:*

| Command Name                  |
|-------------------------------|
| `--prune` / `-P` / `/P`       |
| `--level` / `-L` / `/L`       |
| `--gitignore` / `-g` / `/G`   |
| `--include` / `-m` / `/M`     |
| `--exclude` / `-I` / `/X`     |
| `--disk-usage` / `-u` / `/DU` |
| `--sort` / `-S` / `/SO`       |
| `--reverse` / `-r` / `/R`     |

### `/G`: Follow `.gitignore`

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
        output.rs
        render.rs
        scan.rs

.gitignore rules applied
1 directories, 12 files
```
