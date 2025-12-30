# `tree++`: 完整参数说明和示例文档

本文档简述 [tree++](https://github.com/Water-Run/treepp) 所支持的全部参数与使用示例。  

## 模拟目录

指令的示例输出基于此模拟目录：

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

> `treepp /F` 与 Windows 原生 `tree /F` 行为完全一致：显示卷头信息、树形结构以及末尾统计信息。直接执行 `treepp` 时亦保持原始语义仅展示目录结构。

## 全局用法

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`：可选，默认为当前目录。
- `<OPTIONS>`：可重复、可混用。支持下表列出的 `--`（GNU）、`-`（短参数）与 `/`（CMD，大小写不敏感）三种形式。

## 指令的具体说明

### `/?`: 显示帮助

**功能：** 显示完整的参数帮助信息。  

**语法：**

```powershell
treepp (--help | -h | /?) [<PATH>]
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

### `/V`: 显示版本

**功能：** 输出当前版本信息。  

**语法：**

```powershell
treepp (--version | -v | /V)
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /V
tree++ version 0.1.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

### `/A`: 使用 ASCII 字符绘制树

**功能：** 以 ASCII 树形字符输出，兼容 `tree /A`。  

**语法：**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
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

### `/F`: 显示文件

**功能：** 在目录树中列出文件条目。  

**语法：**

```powershell
treepp (--files | -f | /F) [<PATH>]
```

**示例（与 `/A` 组合）：**

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

### `/FP`: 显示完整路径

**功能：** 以绝对路径展示所有条目。  

**语法：**

```powershell
treepp (--full-path | -p | /FP) [<PATH>]
```

**示例（与 `/F` 组合）：**

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

### `/HR`: 人类可读文件大小

**功能：** 将文件大小转换为 B/KB/MB 等易读单位，常与 `/S` 搭配。  

**语法：**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**示例（`/S /HR /F`）：**

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

### `/S`: 显示文件大小（字节）

**功能：** 显示文件字节数，可与 `/HR` 联用。  

**语法：**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /S /F
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

1 个目录, 12 个文件
```

### `/NI`: 不显示树形连接线

**功能：** 用缩进取代树形符号。  

**语法：**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**示例：**

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

### `/R`: 逆序排序

**功能：** 将当前排序结果倒序输出，可与 `/SO` 组合。  

**语法：**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**示例：**

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

### `/DT`: 显示最后修改日期

**功能：** 在条目后追加文件/目录的最后修改时间。  

**语法：**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**示例：**

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

### `/X`: 排除匹配项

**功能：** 忽略与模式匹配的文件或目录。  

**语法：**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**示例（排除 `*.md`）：**

```powershell
PS D:\Rust\tree++> treepp /F /X *.md
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

### `/L`: 限制递归深度

**功能：** 指定最大递归层级。  

**语法：**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**示例（仅展示 1 层）：**

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

### `/M`: 仅显示匹配项

**功能：** 只保留符合模式的条目。  

**语法：**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**示例（仅显示 `*.rs`）：**

```powershell
PS D:\Rust\tree++> treepp /F /M *.rs
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

### `/Q`: 引号包裹文件名

**功能：** 以双引号包裹路径，方便复制或脚本处理。  

**语法：**

```powershell
treepp (--quote | -q | /Q) [<PATH>]
```

**示例：**

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

### `/DF`: 目录优先显示

**功能：** 输出时先列目录，再列文件。  

**语法：**

```powershell
treepp (--dirs-first | -D | /DF) [<PATH>]
```

**示例：**

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

### `/DU`: 显示目录累计大小

**功能：** 统计每个目录的累计磁盘用量，可与 `/HR` 配合。  

**语法：**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /DU /HR
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
src             31.5 KB
```

### `/IC`: 匹配时忽略大小写

**功能：** 使 `/M`、`/X` 等匹配指令忽略大小写。  

**语法：**

```powershell
treepp (--ignore-case | -c | /IC) [<PATH>]
```
**示例（`/F /M *.MD /IC`）：**
```powershell
PS D:\Rust\tree++> treepp /F /M *.MD /IC
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:\Rust\tree++
│  OPTIONS-zh.md
│  OPTIONS.md
│  README-zh.md
│  README.md

0 个目录, 4 个文件
```

### `/NR`: 不显示末尾统计信息

**功能：** 省略“X 个目录, Y 个文件”汇总。  

**语法：**

```powershell
treepp (--no-report | -n | /NR) [<PATH>]
```

**示例：**

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

### `/P`: 修剪空目录

**功能：** 隐藏不包含内容的目录节点。  

**语法：**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**示例：**

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

### `/SO`: 指定排序方式

**功能：** 依据 `name`、`size`、`mtime` 等字段进行排序，可与 `/R` 组合。

**语法：**

```powershell
treepp (--sort | -S | /SO) <KEY> [<PATH>]
```

**示例（`/F /SO name`）：**

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

*可选的排序字段及说明:*  

| 字段      | 说明        |  
|---------|-----------|  
| `name`  | 按文件名字母表升序 |
| `size`  | 按文件大小升序   |
| `mtime` | 按修改时间升序   |
| `ctime` | 按创建时间升序   |

### `/NH`: 不显示卷信息与头部报告

**功能：** 省略卷名、卷序列号等头部内容。  

**语法：**

```powershell
treepp (--no-header | -N | /NH) [<PATH>]
```

**示例：**

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

### `/SI`: 终端静默

**功能：** 禁止向标准输出写入结果，一般与 `/O` 搭配，静默写入文件。  

**语法：**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**示例（`/F /O tree.json /SI`）：**

```powershell
PS D:\Rust\tree++> treepp /F /O tree.json /SI
PS D:\Rust\tree++>
```

### `/O`: 输出到文件

**功能：** 将结果持久化到 `.txt` / `.json` / `.yml` / `.toml` 文件。默认仍在控制台输出，可配合 `/SI` 静默。  

**语法：**

```powershell
treepp (--output | -o | /O) <FILE.{txt|json|yml|toml}> [<PATH>]
```

**示例：**

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

### `/T`: 扫描线程数

**功能：** 指定扫描线程数量，默认为 4。  

**语法：**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**示例：**

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

### `/MFT`: 使用 MFT（管理员模式）

**功能：** 在管理员权限下显式启用 NTFS MFT 扫描，绕过常规目录遍历，显著提升大目录性能。需要以管理员权限运行。  

> 建议结合[Sudo For Windows](https://learn.microsoft.com/zh-cn/windows/advanced-settings/sudo/)  

**语法：**

```powershell
sudo treepp [<PATH>] (--mft | -M | /MFT) [<OPTIONS>...]
```

**示例：**

```powershell
PS D:\Rust\tree++> sudo treepp /F /MFT
[MFT] enabled: scanning via NTFS Master File Table
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

*需要注意的是，在 MFT 模式下，不可使用以下命令，否则将抛出异常：*

| 命令名称                          |
|-------------------------------|
| `--prune` / `-P` / `/P`       |
| `--level` / `-L` / `/L`       |
| `--gitignore` / `-g` / `/G`   |
| `--include` / `-m` / `/M`     |
| `--exclude` / `-I` / `/X`     |
| `--disk-usage` / `-u` / `/DU` |
| `--sort` / `-S` / `/SO`       |
| `--reverse` / `-r` / `/R`     |

### `/G`: 遵循 `.gitignore`

**功能：** 解析每级目录中的 `.gitignore` 文件，自动忽略匹配条目。  

**语法：**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**示例：**

```powershell
PS D:\Rust\tree++> treepp /F /G
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

.gitignore rules applied
1 个目录, 12 个文件
```
