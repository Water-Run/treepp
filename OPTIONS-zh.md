# `tree++`: 完整参数说明和示例文档

本文档简述 [tree++](https://github.com/Water-Run/treepp) 所支持的全部参数和使用示例.

## 模拟目录

以下指令的示例输出基于此模拟目录:

```powershell
PS D:\数据\Rust\tree++> treepp /f
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

> 可以看到, `treepp /f` 的行为与 Windows 原生 `tree /f` 完全一致. 单纯执行 `treepp` 时亦保持原始语义.

## 指令的具体说明

### `/?`: 显示帮助

**功能:**

显示完整的参数帮助信息.

**形式:**

`--help` `-h` `/?` `/H` `-Help`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /?
tree++ - a better tree command for Windows
Usage:
  treepp [path] [options]
...
```

---

### `/V`: 获取版本

**功能:**

输出当前 `tree++` 的版本号.

**形式:**

`--version` `-v` `/V` `-Version`

**示例:**

```powershell
PS C:\Users\linzh> treepp /v
tree++ version 1.0.0
link: https://github.com/Water-Run/treepp
```

---

### `/A`: 使用 ASCII 字符

**功能:**

使用 ASCII 字符绘制树形结构(兼容 Windows 原生 `tree /A` 的输出风格).

**形式:**

`--ascii` `-A` `/A` `-Ascii`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /A
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
+---.release
\---src
```

---

### `/F`: 显示文件

**功能:**

在目录树中显示文件.

**形式:**

`--files` `-f` `/F` `-Files`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /A /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/FP`: 显示完整路径

**功能:**

以完整路径形式显示文件和目录.

**形式:**

`--full-path` `-p` `/FP` `-FullPath`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /fp
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\数据\Rust\tree++
│  D:\数据\Rust\tree++\.gitignore
│  D:\数据\Rust\tree++\Cargo.toml
│  D:\数据\Rust\tree++\LICENSE
│  D:\数据\Rust\tree++\OPTIONS-zh.md
│  D:\数据\Rust\tree++\OPTIONS.md
│  D:\数据\Rust\tree++\README-zh.md
│  D:\数据\Rust\tree++\README.md
│
├─D:\数据\Rust\tree++\.release
└─D:\数据\Rust\tree++\src
        D:\数据\Rust\tree++\src\engine.rs
        D:\数据\Rust\tree++\src\input.rs
        D:\数据\Rust\tree++\src\main.rs
        D:\数据\Rust\tree++\src\output.rs
```

---

### `/S`: 显示文件大小

**功能:**

显示文件大小, 单位为字节.

**形式:**

`--size` `-s` `/S` `-Size`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f -s
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/HR`: 人类可读文件大小

**功能:**

以人类可读方式显示文件大小(如 B, KB, MB). 通常与 `--size` 组合使用, 也可在显示文件大小时直接生效.

**形式:**

`--human-readable` `-H` `/HR` `-HumanReadable`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f -H
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/NI`: 不显示树形连接线

**功能:**

不显示树形连接线, 以无连接线形式输出结果.

**形式:**

`--no-indent` `-i` `/NI` `-NoIndent`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /ni
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/R`: 逆序排序

**功能:**

对当前排序结果进行逆序输出.

**形式:**

`--reverse` `-r` `/R` `-Reverse`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /r
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/DT`: 显示最后修改日期

**功能:**

显示文件和目录的最后修改时间.

**形式:**

`--date` `-D` `/DT` `-Date`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /dt
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/X`: 排除匹配文件

**功能:**

排除匹配指定模式的文件或目录(常用于忽略构建产物、依赖目录等).

**形式:**

`--exclude <pattern>` `-I <pattern>` `/X <pattern>` `-Exclude <pattern>`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /x "*.md"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/L`: 限制递归深度

**功能:**

限制目录递归的最大深度.

**形式:**

`--level <level>` `-L <level>` `/L <level>` `-Level <level>`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /l 1
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/M`: 仅显示匹配文件

**功能:**

仅显示匹配指定模式的文件或目录.

**形式:**

`--include <pattern>` `-P <pattern>` `/M <pattern>` `-Include <pattern>`

> 说明: 该参数组在 Unix 风格下使用 `-P <pattern>`. 由于 `--prune` 也提供 `-P`(无参数) 的写法, 两者可通过是否携带 `<pattern>` 来区分:
>
> * `-P "*.rs"` 表示 include
> * `-P` 表示 prune

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /m "*.rs"
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
│
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/Q`: 引号包裹文件名

**功能:**

使用双引号包裹文件名输出(便于复制粘贴, 或用于后续脚本处理).

**形式:**

`--quote` `-Q` `/Q` `-Quote`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /q
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/O`: 目录优先显示

**功能:**

在排序与展示时将目录优先于文件显示.

**形式:**

`--dirs-first` `-O` `/O` `-DirsFirst`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /o
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/DU`: 显示目录累计大小

**功能:**

显示目录的累计磁盘占用大小(通常与 `--human-readable` 配合使用更直观).

**形式:**

`--du` `-u` `/DU` `-DiskUsage`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /du
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
.release        0 B
src             18.5 KB
```

---

### `/IC`: 匹配时忽略大小写

**功能:**

匹配时忽略大小写(影响 include 与 exclude 的匹配).

**形式:**

`--ignore-case` `-iC` `/IC` `-IgnoreCase`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /m "*.MD" /ic
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

**形式:**

`--no-report` `-N` `/NR` `-NoReport`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /nr
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/P`: 修剪空目录

**功能:**

修剪空目录, 不显示不包含任何内容的目录.

**形式:**

`--prune` `-P` `/P` `-Prune`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /p
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
└─src
        engine.rs
        input.rs
        main.rs
        output.rs
```

---

### `/SORT`: 指定排序方式

**功能:**

指定排序方式(如 `name`, `size`, `mtime` 等). 可与 `--reverse` 组合以实现倒序.

**形式:**

`--sort <key>` `-S <key>` `/SORT <key>` `-Sort <key>`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /sort size
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/NH`: 不显示卷信息与头部报告

**功能:**

不显示卷信息与头部报告信息(例如 "卷 ... 的文件夹 PATH 列表" 与卷序列号). 适合在脚本或需要纯输出时使用.

**形式:**

`--no-header` `-NH` `/NH` `-NoHeader`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /nh
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

### `/SI`: 静默模式

**功能:**

不输出任何内容到标准输出(通常用于配合 `--output` 将结果写入文件, 同时避免控制台产生输出).

**形式:**

`--silent` `-SI` `/SI` `-Silent`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /out tree.json /si
PS D:\数据\Rust\tree++>
```

---

### `/OUT`: 输出到文件

**功能:**

将结果输出到指定文件(支持 `.txt`, `.json`, `.yml`, `.toml`). 默认情况下仍会在控制台输出结果; 若希望只写文件不在控制台输出, 请配合 `--silent` 使用.

**形式:**

`--output <file>` `-o <file>` `/OUT <file>` `-Output <file>`

**示例:**

```powershell
PS D:\数据\Rust\tree++> treepp /f /out tree.json
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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
        
output: D:\数据\Rust\tree++\tree.json
```

```powershell
PS D:\数据\Rust\tree++> treepp /f /out tree.json /si
PS D:\数据\Rust\tree++>
```
