# `tree++`: 完整参数说明和示例文档

本文档简述 [tree++](https://github.com/Water-Run/treepp) 所支持的全部参数与使用示例。

## 模拟目录

以下指令的示例输出基于此模拟目录： 

```powershell
PS D:\Rust\tree++> treepp /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

> 可以看到，`treepp /F` 的行为与 Windows 原生 `tree /F` 保持一致：展示卷头信息、树形结构与末尾统计信息。单纯执行 `treepp` 时亦保持原始语义（仅显示目录）。

---

## 全局用法

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

* `<PATH>`：可选。默认当前目录。
* `<OPTIONS>`：可重复、可混用。支持 Unix 风格、CMD 风格、PowerShell 风格的等价写法。

---

## 指令的具体说明

### `/?`: 显示帮助

**功能：**
显示完整的参数帮助信息。

**语法：**

```powershell
treepp (--help | -h | /? | -Help) [<PATH>]
```

**示例：**

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

### `/V`: 显示版本

**功能：**
输出当前 `tree++` 的版本信息。

**语法：**

```powershell
treepp (--version | -v | /V | -Version)
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /V
tree++ version 1.0.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

---

### `/A`: 使用 ASCII 字符绘制树

**功能：**
使用 ASCII 字符绘制树形结构（兼容 Windows 原生 `tree /A` 的输出风格）。

**语法：**

```powershell
treepp (--ascii | -a | /A | -Ascii) [<PATH>]
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /A
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
\---src

1 个目录
```

---

### `/F`: 显示文件

**功能：**
在目录树中显示文件。

**语法：**

```powershell
treepp (--files | -f | /F | -Files) [<PATH>]
```

**示例（混合指令集：`/A` + `/F`）：**

```powershell
PS D:\Rust\tree++> treepp /A /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/FP`: 显示完整路径

**功能：**
以完整路径形式显示文件和目录。

**语法：**

```powershell
treepp (--full-path | -p | /FP | -FullPath) [<PATH>]
```

**示例（混合指令集：`/F` + `/FP`）：**

```powershell
PS D:\Rust\tree++> treepp /F /FP
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/S`: 显示文件大小（字节）

**功能：**
显示文件大小（单位：字节）。通常与 `--files` 组合使用以显示文件条目的大小。

**语法：**

```powershell
treepp (--size | -s | /S | -Size) [<PATH>]
```

**示例（混合指令集：`/S` + `/F`）：**

```powershell
PS D:\Rust\tree++> treepp /S /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/HR`: 人类可读文件大小

**功能：**
以人类可读方式显示文件大小（如 B、KB、MB）。常与 `--size`/`/S` 联用。

**语法：**

```powershell
treepp (--human-readable | -H | /HR | -HumanReadable) [<PATH>]
```

**示例（混合指令集：`/S` + `/HR` + `/F`）：**

```powershell
PS D:\Rust\tree++> treepp /S /HR /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/NI`: 不显示树形连接线

**功能：**
不显示树形连接线，以纯缩进形式输出结果。

**语法：**

```powershell
treepp (--no-indent | -i | /NI | -NoIndent) [<PATH>]
```

**示例（混合指令集：`/F` + `/NI`）：**

```powershell
PS D:\Rust\tree++> treepp /F /NI
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/R`: 逆序排序

**功能：**
对当前排序结果进行逆序输出。常与 `--sort`/`/SO` 组合使用。

**语法：**

```powershell
treepp (--reverse | -r | /R | -Reverse) [<PATH>]
```

**示例（混合指令集：`/F` + `/R`）：**

```powershell
PS D:\Rust\tree++> treepp /F /R
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/DT`: 显示最后修改日期

**功能：**
显示文件和目录的最后修改时间。

**语法：**

```powershell
treepp (--date | -d | /DT | -Date) [<PATH>]
```

**示例（混合指令集：`/F` + `/DT`）：**

```powershell
PS D:\Rust\tree++> treepp /F /DT
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/X`: 排除匹配项

**功能：**
排除匹配指定模式的文件或目录（常用于忽略构建产物、依赖目录等）。

**语法：**

```powershell
treepp (--exclude | -I | /X | -Exclude) <PATTERN> [<PATH>]
```

**示例（排除所有 Markdown 文件）：**

```powershell
PS D:\Rust\tree++> treepp /F /X "*.md"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 8 个文件
```

---

### `/L`: 限制递归深度

**功能：**
限制目录递归的最大深度。

**语法：**

```powershell
treepp (--level | -L | /L | -Level) <LEVEL> [<PATH>]
```

**示例（仅展示 1 层深度）：**

```powershell
PS D:\Rust\tree++> treepp /F /L 1
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
│  Cargo.toml
│  LICENSE
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
│
└─src

1 个目录, 6 个文件
```

---

### `/M`: 仅显示匹配项

**功能：**
仅显示匹配指定模式的文件或目录。

**语法：**

```powershell
treepp (--include | -m | /M | -Include) <PATTERN> [<PATH>]
```

**示例（仅显示 Rust 源文件）：**

```powershell
PS D:\Rust\tree++> treepp /F /M "*.rs"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
│
└─src
        cli.rs
        config.rs
        main.rs
        output.rs
        render.rs
        scan.rs

1 个目录, 6 个文件
```

---

### `/Q`: 引号包裹文件名

**功能：**
使用双引号包裹文件名输出（便于复制粘贴，或用于后续脚本处理）。

**语法：**

```powershell
treepp (--quote | -q | /Q | -Quote) [<PATH>]
```

**示例（混合指令集：`/F` + `/Q`）：**

```powershell
PS D:\Rust\tree++> treepp /F /Q
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/DF`: 目录优先显示

**功能：**
在排序与展示时将目录优先于文件显示。

**语法：**

```powershell
treepp (--dirs-first | -D | /DF | -DirsFirst) [<PATH>]
```

**示例（混合指令集：`/F` + `/DF`）：**

```powershell
PS D:\Rust\tree++> treepp /F /DF
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/DU`: 显示目录累计大小

**功能：**
显示目录的累计磁盘占用大小（通常与 `--human-readable`/`/HR` 配合使用更直观）。

**语法：**

```powershell
treepp (--disk-usage | -u | /DU | -DiskUsage) [<PATH>]
```

**示例（混合指令集：`/DU` + `/HR`）：**

```powershell
PS D:\Rust\tree++> treepp /DU /HR
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
src             31.5 KB
```

---

### `/IC`: 匹配时忽略大小写

**功能：**
匹配时忽略大小写（影响 `--include`/`--exclude` 的匹配）。

**语法：**

```powershell
treepp (--ignore-case | -c | /IC | -IgnoreCase) [<PATH>]
```

**示例（混合指令集：`/F` + `/M` + `/IC`，匹配 `*.MD`）：**

```powershell
PS D:\Rust\tree++> treepp /F /M "*.MD" /IC
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md

0 个目录, 4 个文件
```

---

### `/NR`: 不显示末尾统计信息

**功能：**
不显示末尾的文件与目录统计信息。

**语法：**

```powershell
treepp (--no-report | -n | /NR | -NoReport) [<PATH>]
```

**示例（混合指令集：`/F` + `/NR`）：**

```powershell
PS D:\Rust\tree++> treepp /F /NR
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/P`: 修剪空目录

**功能：**
修剪空目录：不显示不包含任何内容的目录。

**语法：**

```powershell
treepp (--prune | -P | /P | -Prune) [<PATH>]
```

**示例（混合指令集：`/P` + `/F`）：**

```powershell
PS D:\Rust\tree++> treepp /P /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

---

### `/SO`: 指定排序方式

**功能：**
指定排序方式（如 `name`、`size`、`mtime` 等）。可与 `--reverse`/`/R` 组合以实现倒序。

**语法：**

```powershell
treepp (--sort | -S | /SO | -Sort) <KEY> [<PATH>]
```

**示例（按名称排序，常见默认即为 name）：**

```powershell
PS D:\Rust\tree++> treepp /F /SO name
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

**示例（混合指令集：`-S mtime` + `/R` + `/F`）：**

```powershell
PS D:\Rust\tree++> treepp -S mtime /R /F
...
```

---

### `/NH`: 不显示卷信息与头部报告

**功能：**
不显示卷信息与头部报告信息（例如“卷…的文件夹 PATH 列表”与“卷序列号为 …”）。适合脚本或需要纯输出时使用。

**语法：**

```powershell
treepp (--no-header | -N | /NH | -NoHeader) [<PATH>]
```

**示例（混合指令集：`/F` + `/NH`）：**

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

1 个目录, 12 个文件
```

---

### `/SI`: 静默模式

**功能：**
终端静默：不输出内容到标准输出。通常用于配合 `--output` 将结果写入文件，同时避免控制台输出。

**语法：**

```powershell
treepp (--silent | -l | /SI | -Silent) [<PATH>]
```

**示例（与输出组合：`/O` + `/SI`）：**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json /SI
PS D:\Rust\tree++>
```

---

### `/O`: 输出到文件

**功能：**
将结果输出至文件（支持 `.txt`, `.json`, `.yml`, `.toml`）。默认情况下仍会在控制台输出结果；若希望只写文件不在控制台输出，请配合 `--silent` 使用。

**语法：**

```powershell
treepp (--output | -o | /O | -Output) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**示例（混合指令集：`/F` + `/O` 输出 JSON）：**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件

output: D:\Rust\tree++\tree.json
```

---

### `/T`: 扫描线程数

**功能：**
设置扫描线程数（默认为 24）。线程数主要影响扫描阶段的并发度；通常不改变输出格式，但会影响大目录下的执行性能。

**语法：**

```powershell
treepp (--thread | -t | /T | -Thread) <N> [<PATH>]
```

**示例（指定 32 线程扫描）：**

```powershell
PS D:\Rust\tree++> treepp /F /T 32
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

### `/NM`: 强制不使用MFT  

**功能：**
强制不使用MFT而是正常扫描，即使在管理员模式下。

**语法：**

```powershell
treepp (--no-mft -nm /NM -NoMFT)
```

**示例（使用Sudo For Windows以管理员权限运行且不使用MFT）：**

```powershell
PS D:\Rust\tree++> sudo treepp /F /NM
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```
