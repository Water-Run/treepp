# `tree++`: Complete Options Reference and Examples

This document summarizes all options supported by [tree++](https://github.com/Water-Run/treepp) and provides usage examples.

## Sample Directory

All example outputs below are based on this simulated directory:

```powershell
PS D:\Data\Rust\tree++> treepp /f
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
├─.release
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

> As shown, `treepp /f` behaves exactly the same as the native Windows `tree /f`. Running `treepp` alone also preserves the original semantics.

## Detailed Option Reference

### `/?`: Show Help

**Description:**

Displays the complete help text for all options.

**Forms:**

`--help` `-h` `/?` `/H` `-Help`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /?
tree++ - a better tree command for Windows
Usage:
  treepp [path] [options]
...
```

---

### `/V`: Show Version

**Description:**

Prints the current `tree++` version.

**Forms:**

`--version` `-v` `/V` `-Version`

**Example:**

```powershell
PS C:\Users\linzh> treepp /v
tree++ version 1.0.0
link: https://github.com/Water-Run/treepp
```

---

### `/A`: Use ASCII Characters

**Description:**

Draws the directory tree using ASCII characters (compatible with the native Windows `tree /A` output style).

**Forms:**

`--ascii` `-A` `/A` `-Ascii`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /A
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
+---.release
\---src
```

---

### `/F`: Show Files

**Description:**

Shows files in the directory tree.

**Forms:**

`--files` `-f` `/F` `-Files`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /A /F
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
|   .gitignore
|   Cargo.toml
|   LICENSE
|   OPTIONS-zh.md
|   OPTIONS.md
|   README-zh.md
|   README.md
|
+---.release
\---src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/FP`: Show Full Paths

**Description:**

Displays files and directories using their full paths.

**Forms:**

`--full-path` `-p` `/FP` `-FullPath`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /fp
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:\Data\Rust\tree++
│  D:\Data\Rust\tree++\.gitignore
│  D:\Data\Rust\tree++\Cargo.toml
│  D:\Data\Rust\tree++\LICENSE
│  D:\Data\Rust\tree++\OPTIONS-zh.md
│  D:\Data\Rust\tree++\OPTIONS.md
│  D:\Data\Rust\tree++\README-zh.md
│  D:\Data\Rust\tree++\README.md
│
├─D:\Data\Rust\tree++\.release
└─D:\Data\Rust\tree++\src
        D:\Data\Rust\tree++\src\engine.rs
        D:\Data\Rust\tree++\src\input.rs
        D:\Data\Rust\tree++\src\main.rs
        D:\Data\Rust\tree++\src\output.rs
```

---

### `/S`: Show File Size

**Description:**

Shows file sizes in bytes.

**Forms:**

`--size` `-s` `/S` `-Size`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f -s
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        38
│  Cargo.toml        512
│  LICENSE           1067
│  OPTIONS-zh.md     4096
│  OPTIONS.md        3840
│  README-zh.md      5120
│  README.md         4864
│
├─.release
└─src
        engine.rs      6144
        input.rs       4096
        main.rs        3584
        output.rs      5120
```

---

### `/HR`: Human-Readable File Sizes

**Description:**

Shows file sizes in a human-readable format (e.g., B, KB, MB). Typically used together with `--size`, but can also take effect directly when showing sizes.

**Forms:**

`--human-readable` `-H` `/HR` `-HumanReadable`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f -H
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        38 B
│  Cargo.toml        512 B
│  LICENSE           1.0 KB
│  OPTIONS-zh.md     4.0 KB
│  OPTIONS.md        3.8 KB
│  README-zh.md      5.0 KB
│  README.md         4.8 KB
│
├─.release
└─src
        engine.rs      6.0 KB
        input.rs       4.0 KB
        main.rs        3.5 KB
        output.rs      5.0 KB
```

---

### `/NI`: Disable Tree Indentation Lines

**Description:**

Disables the tree connector lines and prints in a plain indented layout.

**Forms:**

`--no-indent` `-i` `/NI` `-NoIndent`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /ni
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
  .gitignore
  Cargo.toml
  LICENSE
  OPTIONS-zh.md
  OPTIONS.md
  README-zh.md
  README.md

  .release
  src
    engine.rs
    input.rs
    main.rs
    output.rs
```

---

### `/R`: Reverse Sort Order

**Description:**

Reverses the current sort order.

**Forms:**

`--reverse` `-r` `/R` `-Reverse`

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
│  .gitignore
│
├─src
│       output.rs
│       main.rs
│       input.rs
│       engine.rs
└─.release
```

---

### `/DT`: Show Last Modified Time

**Description:**

Shows the last modified time of files and directories.

**Forms:**

`--date` `-D` `/DT` `-Date`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /dt
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore        2025-12-01 10:12:17
│  Cargo.toml        2025-12-02 18:40:00
│  LICENSE           2024-11-03 09:00:29
│  OPTIONS-zh.md     2025-12-15 14:20:16
│  OPTIONS.md        2025-12-15 14:18:05
│  README-zh.md      2025-12-16 09:30:03
│  README.md         2025-12-16 09:25:38
│
├─.release
└─src
        engine.rs      2025-12-10 21:11:11
        input.rs       2025-12-10 21:05:09
        main.rs        2025-12-10 20:58:47
        output.rs      2025-12-10 21:20:58
```

---

### `/X`: Exclude Matching Files

**Description:**

Excludes files or directories that match a given pattern (commonly used to ignore build artifacts, dependency folders, etc.).

**Forms:**

`--exclude <pattern>` `-I <pattern>` `/X <pattern>` `-Exclude <pattern>`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /x "*.md"
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│
├─.release
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/L`: Limit Recursion Depth

**Description:**

Limits the maximum recursion depth when traversing directories.

**Forms:**

`--level <level>` `-L <level>` `/L <level>` `-Level <level>`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /l 1
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
├─.release
└─src
```

---

### `/M`: Include Only Matching Files

**Description:**

Shows only files or directories that match a given pattern.

**Forms:**

`--include <pattern>` `-P <pattern>` `/M <pattern>` `-Include <pattern>`

> Note: In Unix-style usage, this option group uses `-P <pattern>`. Since `--prune` also supports `-P` (without an argument), you can distinguish them by whether `<pattern>` is present:
>
> * `-P "*.rs"` means include
> * `-P` means prune

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /m "*.rs"
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/Q`: Quote File Names

**Description:**

Wraps file names in double quotes (useful for copy/paste or downstream script processing).

**Forms:**

`--quote` `-Q` `/Q` `-Quote`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /q
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  ".gitignore"
│  "Cargo.toml"
│  "LICENSE"
│  "OPTIONS-zh.md"
│  "OPTIONS.md"
│  "README-zh.md"
│  "README.md"
│
├─".release"
└─"src"
        "engine.rs"
        "input.rs"
        "main.rs"
        "output.rs"
```

---

### `/O`: Show Directories First

**Description:**

Displays directories before files when sorting and rendering.

**Forms:**

`--dirs-first` `-O` `/O` `-DirsFirst`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /o
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
├─.release
├─src
│       engine.rs
│       input.rs
│       main.rs
│       output.rs
│
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

---

### `/DU`: Show Directory Disk Usage Totals

**Description:**

Shows the total disk usage for each directory (typically more readable when combined with `--human-readable`).

**Forms:**

`--du` `-u` `/DU` `-DiskUsage`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /du
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
.release        0 B
src             18.5 KB
```

---

### `/IC`: Case-Insensitive Matching

**Description:**

Makes matching case-insensitive (affects include and exclude matching).

**Forms:**

`--ignore-case` `-iC` `/IC` `-IgnoreCase`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /m "*.MD" /ic
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

---

### `/NR`: Disable Summary Report

**Description:**

Disables the ending file/directory summary report (if the current output would include a summary).

**Forms:**

`--no-report` `-N` `/NR` `-NoReport`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /nr
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
├─.release
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/P`: Prune Empty Directories

**Description:**

Prunes empty directories and hides directories that do not contain any content.

**Forms:**

`--prune` `-P` `/P` `-Prune`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /p
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/SORT`: Specify Sort Key

**Description:**

Specifies the sort key (e.g., `name`, `size`, `mtime`). Can be combined with `--reverse` for descending order.

**Forms:**

`--sort <key>` `-S <key>` `/SORT <key>` `-Sort <key>`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /sort size
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS.md
│  OPTIONS-zh.md
│  README.md
│  README-zh.md
│
├─.release
└─src
        main.rs
        input.rs
        output.rs
        engine.rs
```

---

### `/NH`: Disable Volume Header and Report Header

**Description:**

Hides the volume information and header report lines (e.g., “Folder PATH listing for volume …” and the volume serial number). Useful for scripts or when you need clean output.

**Forms:**

`--no-header` `-NH` `/NH` `-NoHeader`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /nh
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
├─.release
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/SI`: Silent Mode

**Description:**

Produces no output to stdout (typically used together with `--output` to write results to a file while keeping the console quiet).

**Forms:**

`--silent` `-SI` `/SI` `-Silent`

**Example:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /out tree.json /si
PS D:\Data\Rust\tree++>
```

---

### `/OUT`: Write Output to File

**Description:**

Writes the result to a specified file (supports `.txt`, `.json`, `.yml`, `.toml`). By default, results are still printed to the console; to write only to a file without console output, combine with `--silent`.

**Forms:**

`--output <file>` `-o <file>` `/OUT <file>` `-Output <file>`

**Examples:**

```powershell
PS D:\Data\Rust\tree++> treepp /f /out tree.json
Folder PATH listing for volume Storage
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
├─.release
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
        
output: D:\Data\Rust\tree++\tree.json
```

```powershell
PS D:\Data\Rust\tree++> treepp /f /out tree.json /si
PS D:\Data\Rust\tree++>
```
