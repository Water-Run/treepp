# `tree++`: Full Options Reference and Examples

This document summarizes all options supported by [tree++](https://github.com/Water-Run/treepp) and provides usage examples.

## Mock Directory

All example outputs below are based on the following mock directory (Windows-style):

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

1 directory, 12 files
```

> As you can see, `treepp /F` behaves consistently with the native Windows `tree /F`: it prints the volume header, the tree layout, and the final summary line. Running `treepp` without options preserves the original semantics (directories only).

---

## Global Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

* `<PATH>`: Optional. Defaults to the current directory.
* `<OPTIONS>`: Repeatable and mixable. Equivalent forms are supported in Unix style, CMD style, and PowerShell style.

---

## Option Reference

### `/?`: Show Help

**Function:**
Show the full help text for all options.

**Syntax:**

```powershell
treepp (--help | -h | /? | -Help) [<PATH>]
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

---

### `/V`: Show Version

**Function:**
Print the version information for `tree++`.

**Syntax:**

```powershell
treepp (--version | -v | /V | -Version)
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /V
tree++ version 1.0.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

---

### `/A`: Draw the Tree Using ASCII

**Function:**
Draw the tree using ASCII characters (compatible with the native Windows `tree /A` style).

**Syntax:**

```powershell
treepp (--ascii | -a | /A | -Ascii) [<PATH>]
```

**Example:**

```powershell
PS D:\Rust\tree++> treepp /A
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
\---src

1 directory
```

---

### `/F`: Show Files

**Function:**
Show files in the directory tree.

**Syntax:**

```powershell
treepp (--files | -f | /F | -Files) [<PATH>]
```

**Example (mixed option styles: `/A` + `/F`):**

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

1 directory, 12 files
```

---

### `/FP`: Show Full Paths

**Function:**
Print directories and files as full paths.

**Syntax:**

```powershell
treepp (--full-path | -p | /FP | -FullPath) [<PATH>]
```

**Example (mixed option styles: `/F` + `/FP`):**

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

1 directory, 12 files
```

---

### `/S`: Show File Sizes (Bytes)

**Function:**
Show file sizes in bytes. Typically used together with `--files` to display file entry sizes.

**Syntax:**

```powershell
treepp (--size | -s | /S | -Size) [<PATH>]
```

**Example (mixed option styles: `/S` + `/F`):**

```powershell
PS D:\Rust\tree++> treepp /S /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  Cargo.toml        982
│  LICENSE           1067
│  OPTIONS-zh.md     8120
│  OPTIONS.md        7644
│  README-zh.md      10420
│  README.md         9288
│
└─src
        cli.rs         6120
        config.rs      2840
        main.rs        1980
        output.rs      7440
        render.rs      5360
        scan.rs        9020

1 directory, 12 files
```

---

### `/HR`: Human-Readable Sizes

**Function:**
Display file sizes in human-readable units (e.g., B, KB, MB). Commonly used with `--size`/`/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR | -HumanReadable) [<PATH>]
```

**Example (mixed option styles: `/S` + `/HR` + `/F`):**

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

1 directory, 12 files
```

---

### `/NI`: Disable Tree Connectors

**Function:**
Hide tree connector lines and print results using plain indentation.

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI | -NoIndent) [<PATH>]
```

**Example (mixed option styles: `/F` + `/NI`):**

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

1 directory, 12 files
```

---

### `/R`: Reverse Order

**Function:**
Reverse the current sort order. Commonly used with `--sort`/`/SO`.

**Syntax:**

```powershell
treepp (--reverse | -r | /R | -Reverse) [<PATH>]
```

**Example (mixed option styles: `/F` + `/R`):**

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

1 directory, 12 files
```

---

### `/DT`: Show Last Modified Time

**Function:**
Show the last modified time for files and directories.

**Syntax:**

```powershell
treepp (--date | -d | /DT | -Date) [<PATH>]
```

**Example (mixed option styles: `/F` + `/DT`):**

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

1 directory, 12 files
```

---

### `/X`: Exclude Matches

**Function:**
Exclude files or directories matching a pattern (commonly used to ignore build outputs, dependency folders, etc.).

**Syntax:**

```powershell
treepp (--exclude | -I | /X | -Exclude) <PATTERN> [<PATH>]
```

**Example (exclude all Markdown files):**

```powershell
PS D:\Rust\tree++> treepp /F /X "*.md"
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

1 directory, 8 files
```

---

### `/L`: Limit Recursion Depth

**Function:**
Limit the maximum recursion depth.

**Syntax:**

```powershell
treepp (--level | -L | /L | -Level) <LEVEL> [<PATH>]
```

**Example (depth 1 only):**

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

1 directory, 6 files
```

---

### `/M`: Include Only Matches

**Function:**
Show only files or directories matching a pattern.

**Syntax:**

```powershell
treepp (--include | -m | /M | -Include) <PATTERN> [<PATH>]
```

**Example (show only Rust source files):**

```powershell
PS D:\Rust\tree++> treepp /F /M "*.rs"
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

1 directory, 6 files
```

---

### `/Q`: Quote Names

**Function:**
Wrap names in double quotes (useful for copy/paste or follow-up script processing).

**Syntax:**

```powershell
treepp (--quote | -q | /Q | -Quote) [<PATH>]
```

**Example (mixed option styles: `/F` + `/Q`):**

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

1 directory, 12 files
```

---

### `/DF`: Directories First

**Function:**
Display directories before files during sorting and rendering.

**Syntax:**

```powershell
treepp (--dirs-first | -D | /DF | -DirsFirst) [<PATH>]
```

**Example (mixed option styles: `/F` + `/DF`):**

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

1 directory, 12 files
```

---

### `/DU`: Show Directory Disk Usage

**Function:**
Show the cumulative disk usage for directories (typically paired with `--human-readable`/`/HR` for readability).

**Syntax:**

```powershell
treepp (--disk-usage | -u | /DU | -DiskUsage) [<PATH>]
```

**Example (mixed option styles: `/DU` + `/HR`):**

```powershell
PS D:\Rust\tree++> treepp /DU /HR
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
src             31.5 KB
```

---

### `/IC`: Case-Insensitive Matching

**Function:**
Ignore case when matching patterns (affects `--include`/`--exclude`).

**Syntax:**

```powershell
treepp (--ignore-case | -c | /IC | -IgnoreCase) [<PATH>]
```

**Example (mixed option styles: `/F` + `/M` + `/IC`, matching `*.MD`):**

```powershell
PS D:\Rust\tree++> treepp /F /M "*.MD" /IC
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Rust\tree++
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md

0 directories, 4 files
```

---

### `/NR`: Disable the Summary Line

**Function:**
Do not print the final summary line (directories/files counts).

**Syntax:**

```powershell
treepp (--no-report | -n | /NR | -NoReport) [<PATH>]
```

**Example (mixed option styles: `/F` + `/NR`):**

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

---

### `/P`: Prune Empty Directories

**Function:**
Prune empty directories (do not show directories that contain nothing).

**Syntax:**

```powershell
treepp (--prune | -P | /P | -Prune) [<PATH>]
```

**Example (mixed option styles: `/P` + `/F`):**

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

1 directory, 12 files
```

---

### `/SO`: Choose Sort Key

**Function:**
Choose a sort key (e.g., `name`, `size`, `mtime`). Can be combined with `--reverse`/`/R` to invert order.

**Syntax:**

```powershell
treepp (--sort | -S | /SO | -Sort) <KEY> [<PATH>]
```

**Example (sort by name; often the default):**

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

1 directory, 12 files
```

**Example (mixed option styles: `-S mtime` + `/R` + `/F`):**

```powershell
PS D:\Rust\tree++> treepp -S mtime /R /F
...
```

---

### `/NH`: Hide Volume Header

**Function:**
Hide the volume header lines (e.g., “Folder PATH listing for volume …” and “Volume serial number is …”). Useful for scripts or when you want clean output.

**Syntax:**

```powershell
treepp (--no-header | -N | /NH | -NoHeader) [<PATH>]
```

**Example (mixed option styles: `/F` + `/NH`):**

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

1 directory, 12 files
```

---

### `/SI`: Silent Mode

**Function:**
Suppress output to stdout. Typically used with `--output` to write results to a file without printing to the console.

**Syntax:**

```powershell
treepp (--silent | -l | /SI | -Silent) [<PATH>]
```

**Example (combined with output: `/O` + `/SI`):**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json /SI
PS D:\Rust\tree++>
```

---

### `/O`: Output to File

**Function:**
Write output to a file (supports `.txt`, `.json`, `.yml`, `.toml`). By default, output is still printed to the console; use `--silent` to write only to the file.

**Syntax:**

```powershell
treepp (--output | -o | /O | -Output) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**Example (mixed option styles: `/F` + `/O` to JSON):**

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

1 directory, 12 files

output: D:\Rust\tree++\tree.json
```

---

### `/T`: Scan Thread Count

**Function:**
Set the scan thread count (default: 24). This affects scanning concurrency and performance for large directories; it typically does not change the output format.

**Syntax:**

```powershell
treepp (--thread | -t | /T | -Thread) <N> [<PATH>]
```

**Example (use 32 scan threads):**

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

1 directory, 12 files
```

---

### `/NM`: Force Disable MFT  

**Description:**  
Forces normal scanning instead of using the MFT, even when running in administrator mode.

**Syntax:**

```powershell
treepp (--no-mft -nm /NM -NoMFT)
```

**Example (run with administrator privileges using Sudo for Windows, without using MFT):**

```powershell
PS D:\Rust\tree++> sudo treepp /F /NM
```
