# `tree++`: Complete Options Reference and Examples

This document briefly describes all options supported by [tree++](https://github.com/Water-Run/treepp) and provides usage examples.

## Mock Directory

The sample outputs below are based on this mock directory:

```powershell
PS D:\数据\zig\tree++> treepp /F
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
````

> As you can see, `treepp /F` behaves exactly like the native Windows `tree /F`. Running `treepp` alone also preserves the original semantics.

---

## Global Usage

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

* `<PATH>`: Optional. Defaults to the current directory.
* `<OPTIONS>`: Repeatable. You may freely mix Unix-style, CMD-style, and PowerShell-style equivalent forms.

---

## Option Details

### `/?`: Show Help

**Function:**

Display full help information.

**Syntax:**

```powershell
treepp (--help | -h | /? | /H | -Help)
```

**Example:**

```powershell
PS D:\数据\zig\tree++> treepp /?
tree++ - a better tree command for Windows
Usage:
  treepp [path] [options]
...
```

---

### `/V`: Show Version

**Function:**

Print the current `tree++` version.

**Syntax:**

```powershell
treepp (--version | -v | /V | -Version)
```

**Example:**

```powershell
PS D:\数据\zig\tree++> treepp /V
tree++ version 1.0.0
link: https://github.com/Water-Run/treepp
```

---

### `/A`: Draw Using ASCII

**Function:**

Draw the tree using ASCII characters (compatible with the native Windows `tree /A` style).

**Syntax:**

```powershell
treepp (--ascii | -A | /A | -Ascii) [<PATH>]
```

**Example:**

```powershell
PS D:\数据\zig\tree++> treepp /A
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
\---src
```

---

### `/F`: Show Files

**Function:**

Show files in the directory tree.

**Syntax:**

```powershell
treepp (--files | -f | /F | -Files) [<PATH>]
```

**Example (mixed option set: `/A` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp /A /F
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
|   .gitignore
|   build.zig
|   build.zig.zon
|   LICENSE
|   OPTIONS-zh.md
|   OPTIONS.md
|   README-zh.md
|   README.md
|
\---src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/FP`: Show Full Paths

**Function:**

Display files and directories using full paths.

**Syntax:**

```powershell
treepp (--full-path | -p | /FP | -FullPath) [<PATH>]
```

**Example (mixed option set: `/F` + `/FP`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /FP
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:\数据\zig\tree++
│  D:\数据\zig\tree++\.gitignore
│  D:\数据\zig\tree++\build.zig
│  D:\数据\zig\tree++\build.zig.zon
│  D:\数据\zig\tree++\LICENSE
│  D:\数据\zig\tree++\OPTIONS-zh.md
│  D:\数据\zig\tree++\OPTIONS.md
│  D:\数据\zig\tree++\README-zh.md
│  D:\数据\zig\tree++\README.md
│
└─D:\数据\zig\tree++\src
        D:\数据\zig\tree++\src\cli.zig
        D:\数据\zig\tree++\src\conf.zig
        D:\数据\zig\tree++\src\fmt.zig
        D:\数据\zig\tree++\src\io.zig
        D:\数据\zig\tree++\src\main.zig
        D:\数据\zig\tree++\src\scan.zig
```

---

### `/S`: Show File Sizes (Bytes)

**Function:**

Show file sizes in bytes. Typically used together with `--files` to display sizes for file entries.

**Syntax:**

```powershell
treepp (--size | -s | /S | -Size) [<PATH>]
```

**Example (mixed option set: `/S` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp /S /F
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore        38
│  build.zig         1024
│  build.zig.zon     256
│  LICENSE           1067
│  OPTIONS-zh.md     4096
│  OPTIONS.md        3840
│  README-zh.md      5120
│  README.md         4864
│
└─src
        cli.zig         2048
        conf.zig        1536
        fmt.zig         3072
        io.zig          4096
        main.zig        2048
        scan.zig        2560
```

**Example (mixed option set: `-s` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp -s /F
...
```

---

### `/HR`: Human-Readable File Sizes

**Function:**

Show file sizes in human-readable units (e.g., B, KB, MB). Commonly used with `--size`/`/S`.

**Syntax:**

```powershell
treepp (--human-readable | -H | /HR | -HumanReadable) [<PATH>]
```

**Example (mixed option set: `/S` + `/HR` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp /S /HR /F
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore        38 B
│  build.zig         1.0 KB
│  build.zig.zon     256 B
│  LICENSE           1.0 KB
│  OPTIONS-zh.md     4.0 KB
│  OPTIONS.md        3.8 KB
│  README-zh.md      5.0 KB
│  README.md         4.8 KB
│
└─src
        cli.zig         2.0 KB
        conf.zig        1.5 KB
        fmt.zig         3.0 KB
        io.zig          4.0 KB
        main.zig        2.0 KB
        scan.zig        2.5 KB
```

---

### `/NI`: Hide Tree Connector Lines

**Function:**

Do not display tree connector lines; output is printed without the connecting glyphs.

**Syntax:**

```powershell
treepp (--no-indent | -i | /NI | -NoIndent) [<PATH>]
```

**Example (mixed option set: `/F` + `/NI`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /NI
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
  .gitignore
  build.zig
  build.zig.zon
  LICENSE
  OPTIONS-zh.md
  OPTIONS.md
  README-zh.md
  README.md

  src
    cli.zig
    conf.zig
    fmt.zig
    io.zig
    main.zig
    scan.zig
```

---

### `/R`: Reverse Order

**Function:**

Reverse the current ordering. Commonly used together with `--sort`/`/SORT`.

**Syntax:**

```powershell
treepp (--reverse | -r | /R | -Reverse) [<PATH>]
```

**Example (mixed option set: `/F` + `/R`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /R
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  README.md
│  README-zh.md
│  OPTIONS.md
│  OPTIONS-zh.md
│  LICENSE
│  build.zig.zon
│  build.zig
│  .gitignore
│
└─src
        scan.zig
        main.zig
        io.zig
        fmt.zig
        conf.zig
        cli.zig
```

---

### `/DT`: Show Last Modified Time

**Function:**

Show the last modified timestamp for files and directories.

**Syntax:**

```powershell
treepp (--date | -D | /DT | -Date) [<PATH>]
```

**Example (mixed option set: `/F` + `/DT`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /DT
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore        2025-12-01 10:12:17
│  build.zig         2025-12-15 18:40:00
│  build.zig.zon     2025-12-15 18:40:00
│  LICENSE           2024-11-03 09:00:29
│  OPTIONS-zh.md     2025-12-15 14:20:16
│  OPTIONS.md        2025-12-15 14:18:05
│  README-zh.md      2025-12-16 09:30:03
│  README.md         2025-12-16 09:25:38
│
└─src
        cli.zig        2025-12-10 21:11:11
        conf.zig       2025-12-10 21:05:09
        fmt.zig        2025-12-10 20:58:47
        io.zig         2025-12-10 21:20:58
        main.zig       2025-12-10 20:58:47
        scan.zig       2025-12-10 21:20:58
```

---

### `/X`: Exclude Matches

**Function:**

Exclude files or directories matching a given pattern (commonly used to ignore build artifacts, dependency folders, etc.).

**Syntax:**

```powershell
treepp (--exclude | -I | /X | -Exclude) <PATTERN> [<PATH>]
```

**Example (mixed option set: `/F` + `/X`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /X "*.md"
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/L`: Limit Recursion Depth

**Function:**

Limit the maximum recursion depth.

**Syntax:**

```powershell
treepp (--level | -L | /L | -Level) <LEVEL> [<PATH>]
```

**Example (mixed option set: `/F` + `/L`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /L 1
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
```

---

### `/M`: Include Only Matches

**Function:**

Show only files or directories matching a given pattern.

**Syntax:**

```powershell
treepp (--include | -m | /M | -Include) <PATTERN> [<PATH>]
```

**Example (mixed option set: `/F` + `/M`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /M "*.zig"
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/Q`: Quote File Names

**Function:**

Wrap file names in double quotes (useful for copy/paste or subsequent script processing).

**Syntax:**

```powershell
treepp (--quote | -Q | /Q | -Quote) [<PATH>]
```

**Example (mixed option set: `/F` + `/Q`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /Q
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  ".gitignore"
│  "build.zig"
│  "build.zig.zon"
│  "LICENSE"
│  "OPTIONS-zh.md"
│  "OPTIONS.md"
│  "README-zh.md"
│  "README.md"
│
└─"src"
        "cli.zig"
        "conf.zig"
        "fmt.zig"
        "io.zig"
        "main.zig"
        "scan.zig"
```

---

### `/O`: Directories First

**Function:**

List directories before files (affects sorting and display order).

**Syntax:**

```powershell
treepp (--dirs-first | -O | /O | -DirsFirst) [<PATH>]
```

**Example (mixed option set: `/F` + `/O`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /O
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
└─src
│       cli.zig
│       conf.zig
│       fmt.zig
│       io.zig
│       main.zig
│       scan.zig
│
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

---

### `/DU`: Show Directory Disk Usage

**Function:**

Show cumulative disk usage for directories (usually more readable when paired with `--human-readable`/`/HR`).

**Syntax:**

```powershell
treepp (--disk-usage | --du | -u | /DU | -DiskUsage) [<PATH>]
```

**Example (mixed option set: `/DU` + `/HR`):**

```powershell
PS D:\数据\zig\tree++> treepp /DU /HR
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
src             18.5 KB
```

---

### `/IC`: Case-Insensitive Matching

**Function:**

Perform case-insensitive matching (affects `--include`/`--exclude`).

**Syntax:**

```powershell
treepp (--ignore-case | -iC | /IC | -IgnoreCase) [<PATH>]
```

**Example (mixed option set: `/F` + `/M` + `/IC`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /M "*.MD" /IC
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

---

### `/NR`: Hide the Summary Report

**Function:**

Do not display the summary report at the end (if the current output includes one).

**Syntax:**

```powershell
treepp (--no-report | -N | /NR | -NoReport) [<PATH>]
```

**Example (mixed option set: `/F` + `/NR`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /NR
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/P`: Prune Empty Directories

**Function:**

Prune empty directories; do not display directories that contain no entries.

**Syntax:**

```powershell
treepp (--prune | -P | /P | -Prune) [<PATH>]
```

**Example (mixed option set: `/P` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp /P /F
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/SORT`: Choose Sorting Key

**Function:**

Choose a sorting key (e.g. `name`, `size`, `mtime`). Can be combined with `--reverse`/`/R` to reverse the order.

**Syntax:**

```powershell
treepp (--sort | -S | /SORT | -Sort) <KEY> [<PATH>]
```

**Example (mixed option set: `/SORT` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /SORT name
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

**Example (mixed option set: `-S` + `/R` + `/F`):**

```powershell
PS D:\数据\zig\tree++> treepp -S mtime /R /F
...
```

---

### `/NH`: Hide Volume Header and Report Header

**Function:**

Hide volume info and header report lines (e.g., `Volume ... PATH listing` and the volume serial number). Useful for scripts or when you need clean output.

**Syntax:**

```powershell
treepp (--no-header | -NH | /NH | -NoHeader) [<PATH>]
```

**Example (mixed option set: `/F` + `/NH`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /NH
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig
```

---

### `/SI`: Silent Mode

**Function:**

Do not print anything to stdout. Typically used with `--save` to write results to a file without producing console output.

**Syntax:**

```powershell
treepp (--silent | -SI | /SI | -Silent) [<PATH>]
```

**Example (with saving: `/SV` + `/SI`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json /SI
PS D:\数据\zig\tree++>
```

---

### `/SV`: Save Output to a File

**Function:**

Save the result to the specified file (supported: `.txt`, `.json`, `.yml`, `.toml`).
By default, output is still printed to the console; to write only to the file, combine with `--silent`.

**Syntax:**

```powershell
treepp (--save | -sv | /SV | -Save) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**Example (mixed option set: `/F` + `/SV`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json
Volume directory PATH listing
Volume serial number is 26E9-52C1
D:.
│  .gitignore
│  build.zig
│  build.zig.zon
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src
        cli.zig
        conf.zig
        fmt.zig
        io.zig
        main.zig
        scan.zig

save: D:\数据\zig\tree++\tree.json
```

**Example (write only, no console output: `/SV` + `/SI`):**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json /SI
PS D:\数据\zig\tree++>
```
