# `tree++`: 完整参数说明和示例文档

本文档简述 [tree++](https://github.com/Water-Run/treepp) 所支持的全部参数与使用示例.

## 模拟目录

以下指令的示例输出基于此模拟目录:

```powershell
PS D:\数据\zig\tree++> treepp /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

> 可以看到, `treepp /F` 的行为与 Windows 原生 `tree /F` 完全一致. 单纯执行 `treepp` 时亦保持原始语义.

---

## 全局用法

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

* `<PATH>`: 可选. 默认当前目录.
* `<OPTIONS>`: 可重复. 选项可混合使用 Unix 风格 / CMD 风格 / PowerShell 风格的等价写法.

---

## 指令的具体说明

### `/?`: 显示帮助

**功能:**

显示完整的参数帮助信息.

**语法:**

```powershell
treepp (--help | -h | /? | /H | -Help)
```

**示例:**

```powershell
PS D:\数据\zig\tree++> treepp /?
tree++ - a better tree command for Windows
Usage:
  treepp [path] [options]
...
```

---

### `/V`: 获取版本

**功能:**

输出当前 `tree++` 的版本号.

**语法:**

```powershell
treepp (--version | -v | /V | -Version)
```

**示例:**

```powershell
PS D:\数据\zig\tree++> treepp /V
tree++ version 1.0.0
link: https://github.com/Water-Run/treepp
```

---

### `/A`: 使用 ASCII 字符绘制树

**功能:**

使用 ASCII 字符绘制树形结构(兼容 Windows 原生 `tree /A` 的输出风格).

**语法:**

```powershell
treepp (--ascii | -A | /A | -Ascii) [<PATH>]
```

**示例:**

```powershell
PS D:\数据\zig\tree++> treepp /A
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
\---src
```

---

### `/F`: 显示文件

**功能:**

在目录树中显示文件.

**语法:**

```powershell
treepp (--files | -f | /F | -Files) [<PATH>]
```

**示例（混合指令集: `/A` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp /A /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/FP`: 显示完整路径

**功能:**

以完整路径形式显示文件和目录.

**语法:**

```powershell
treepp (--full-path | -p | /FP | -FullPath) [<PATH>]
```

**示例（混合指令集: `/F` + `/FP`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /FP
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/S`: 显示文件大小（字节）

**功能:**

显示文件大小, 单位为字节. 通常与 `--files` 组合使用以显示文件条目的大小.

**语法:**

```powershell
treepp (--size | -s | /S | -Size) [<PATH>]
```

**示例（混合指令集: `/S` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp /S /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

**示例（混合指令集: `-s` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp -s /F
...
```

---

### `/HR`: 人类可读文件大小

**功能:**

以人类可读方式显示文件大小(如 B, KB, MB). 常与 `--size`/`/S` 联用.

**语法:**

```powershell
treepp (--human-readable | -H | /HR | -HumanReadable) [<PATH>]
```

**示例（混合指令集: `/S` + `/HR` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp /S /HR /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/NI`: 不显示树形连接线

**功能:**

不显示树形连接线, 以无连接线形式输出结果.

**语法:**

```powershell
treepp (--no-indent | -i | /NI | -NoIndent) [<PATH>]
```

**示例（混合指令集: `/F` + `/NI`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /NI
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/R`: 逆序排序

**功能:**

对当前排序结果进行逆序输出. 常与 `--sort`/`/SORT` 组合使用.

**语法:**

```powershell
treepp (--reverse | -r | /R | -Reverse) [<PATH>]
```

**示例（混合指令集: `/F` + `/R`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /R
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/DT`: 显示最后修改日期

**功能:**

显示文件和目录的最后修改时间.

**语法:**

```powershell
treepp (--date | -D | /DT | -Date) [<PATH>]
```

**示例（混合指令集: `/F` + `/DT`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /DT
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/X`: 排除匹配项

**功能:**

排除匹配指定模式的文件或目录(常用于忽略构建产物、依赖目录等).

**语法:**

```powershell
treepp (--exclude | -I | /X | -Exclude) <PATTERN> [<PATH>]
```

**示例（混合指令集: `/F` + `/X`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /X "*.md"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/L`: 限制递归深度

**功能:**

限制目录递归的最大深度.

**语法:**

```powershell
treepp (--level | -L | /L | -Level) <LEVEL> [<PATH>]
```

**示例（混合指令集: `/F` + `/L`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /L 1
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/M`: 仅显示匹配项

**功能:**

仅显示匹配指定模式的文件或目录.

**语法:**

```powershell
treepp (--include | -m | /M | -Include) <PATTERN> [<PATH>]
```

**示例（混合指令集: `/F` + `/M`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /M "*.zig"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/Q`: 引号包裹文件名

**功能:**

使用双引号包裹文件名输出(便于复制粘贴, 或用于后续脚本处理).

**语法:**

```powershell
treepp (--quote | -Q | /Q | -Quote) [<PATH>]
```

**示例（混合指令集: `/F` + `/Q`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /Q
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/O`: 目录优先显示

**功能:**

在排序与展示时将目录优先于文件显示.

**语法:**

```powershell
treepp (--dirs-first | -O | /O | -DirsFirst) [<PATH>]
```

**示例（混合指令集: `/F` + `/O`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /O
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/DU`: 显示目录累计大小

**功能:**

显示目录的累计磁盘占用大小(通常与 `--human-readable`/`/HR` 配合使用更直观).

**语法:**

```powershell
treepp (--disk-usage | --du | -u | /DU | -DiskUsage) [<PATH>]
```

**示例（混合指令集: `/DU` + `/HR`）:**

```powershell
PS D:\数据\zig\tree++> treepp /DU /HR
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
src             18.5 KB
```

---

### `/IC`: 匹配时忽略大小写

**功能:**

匹配时忽略大小写(影响 `--include`/`--exclude` 的匹配).

**语法:**

```powershell
treepp (--ignore-case | -iC | /IC | -IgnoreCase) [<PATH>]
```

**示例（混合指令集: `/F` + `/M` + `/IC`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /M "*.MD" /IC
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md
```

---

### `/NR`: 不显示统计信息

**功能:**

不显示末尾的文件与目录统计信息(若当前输出包含统计尾部).

**语法:**

```powershell
treepp (--no-report | -N | /NR | -NoReport) [<PATH>]
```

**示例（混合指令集: `/F` + `/NR`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /NR
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/P`: 修剪空目录

**功能:**

修剪空目录, 不显示不包含任何内容的目录.

**语法:**

```powershell
treepp (--prune | -P | /P | -Prune) [<PATH>]
```

**示例（混合指令集: `/P` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp /P /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/SORT`: 指定排序方式

**功能:**

指定排序方式(如 `name`, `size`, `mtime` 等). 可与 `--reverse`/`/R` 组合以实现倒序.

**语法:**

```powershell
treepp (--sort | -S | /SORT | -Sort) <KEY> [<PATH>]
```

**示例（混合指令集: `/SORT` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /SORT name
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

**示例（混合指令集: `-S` + `/R` + `/F`）:**

```powershell
PS D:\数据\zig\tree++> treepp -S mtime /R /F
...
```

---

### `/NH`: 不显示卷信息与头部报告

**功能:**

不显示卷信息与头部报告信息(例如 "卷 ... 的文件夹 PATH 列表" 与卷序列号). 适合在脚本或需要纯输出时使用.

**语法:**

```powershell
treepp (--no-header | -NH | /NH | -NoHeader) [<PATH>]
```

**示例（混合指令集: `/F` + `/NH`）:**

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

### `/SI`: 静默模式

**功能:**

不输出任何内容到标准输出. 通常用于配合 `--save` 将结果写入文件, 同时避免控制台产生输出.

**语法:**

```powershell
treepp (--silent | -SI | /SI | -Silent) [<PATH>]
```

**示例（与保存组合: `/SV` + `/SI`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json /SI
PS D:\数据\zig\tree++>
```

---

### `/SV`: 保存输出到文件

**功能:**

将结果保存到指定文件(支持 `.txt`, `.json`, `.yml`, `.toml`).
默认情况下仍会在控制台输出结果; 若希望只写文件不在控制台输出, 请配合 `--silent` 使用.

**语法:**

```powershell
treepp (--save | -sv | /SV | -Save) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**示例（混合指令集: `/F` + `/SV`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

**示例（仅写文件不输出: `/SV` + `/SI`）:**

```powershell
PS D:\数据\zig\tree++> treepp /F /SV tree.json /SI
PS D:\数据\zig\tree++>
```
