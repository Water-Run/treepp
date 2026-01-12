# `tree++`: 完整参数说明和示例文档

本文档简述 [tree++](https://github.com/Water-Run/treepp) 所支持的全部参数与使用示例。

## 模拟目录

指令的示例输出基于此模拟目录：

```powershell
PS D:\数据\Rust\tree++> treepp /f
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

> `treepp /F` 与 Windows 原生 `tree /F` 行为完全一致（diff级别）：显示卷头信息和树形结构。直接执行 `treepp` 时亦保持原始语义仅展示目录结构。

## 全局用法

```powershell
treepp [<PATH>] [<OPTIONS>...]
```

- `<PATH>`：可选，默认为当前目录。未指定路径时根路径显示为 `X:.` 格式；显式指定路径时显示为完整大写路径。
- `<OPTIONS>`：可重复、可混用。支持下表列出的 `--`（GNU，大小写敏感）、`-`（短参数，大小写敏感）与 `/`（CMD，大小写不敏感）三种形式。

## 输出模式说明

`tree++` 支持两种输出模式：

### 流式输出（默认）

边扫描边渲染边输出，实现实时滚动效果。适用于大多数交互式场景。

### 批处理模式

通过 `--batch`（`-b` / `/B`）显式启用。完整扫描后再统一输出。以下功能**需要批处理模式**：

- 结构化输出格式（JSON、YAML、TOML）
- `/DU`（目录累计大小，需要完整树计算）
- `/T`（多线程扫描）

## 指令的具体说明

### `/?`: 显示帮助

**功能：** 显示完整的参数帮助信息。

**语法：**

```powershell
treepp (--help | -h | /?)
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /?
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

### `/V`: 显示版本

**功能：** 输出当前版本信息。

**语法：**

```powershell
treepp (--version | -v | /V)
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /v
tree++ version 0.1.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

### `/B`: 批处理模式

**功能：** 启用批处理模式，完整扫描后再统一输出。某些功能（如结构化输出、磁盘用量计算、多线程扫描）需要此模式。

**语法：**

```powershell
treepp (--batch | -b | /B) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /b /f /du
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/A`: 使用 ASCII 字符绘制树

**功能：** 以 ASCII 树形字符输出，兼容 `tree /A`。

**语法：**

```powershell
treepp (--ascii | -a | /A) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /a
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
\---src
```

### `/F`: 显示文件

**功能：** 在目录树中列出文件条目。

**语法：**

```powershell
treepp (--files | -f | /F) [<PATH>]
```

**示例（与 `/A` 组合）：**

```powershell
PS D:\数据\Rust\tree++> treepp /a /f
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/FP`: 显示完整路径

**功能：** 以绝对路径展示所有条目。

**语法：**

```powershell
treepp (--full-path | -p | /FP) [<PATH>]
```

**示例（与 `/F` 组合）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /fp
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
│  D:\数据\Rust\tree++\.gitignore
│  D:\数据\Rust\tree++\Cargo.lock
│  D:\数据\Rust\tree++\Cargo.toml
│  D:\数据\Rust\tree++\LICENSE
│  D:\数据\Rust\tree++\OPTIONS-zh.md
│  D:\数据\Rust\tree++\OPTIONS.md
│  D:\数据\Rust\tree++\README-zh.md
│  D:\数据\Rust\tree++\README.md
│
└─D:\数据\Rust\tree++\src
        D:\数据\Rust\tree++\src\cli.rs
        D:\数据\Rust\tree++\src\config.rs
        D:\数据\Rust\tree++\src\error.rs
        D:\数据\Rust\tree++\src\main.rs
        D:\数据\Rust\tree++\src\output.rs
        D:\数据\Rust\tree++\src\render.rs
        D:\数据\Rust\tree++\src\scan.rs

```

### `/HR`: 人类可读文件大小

**功能：** 将文件大小转换为 B/KB/MB/GB/TB 等易读单位。启用此选项会自动启用 `/S`。

**语法：**

```powershell
treepp (--human-readable | -H | /HR) [<PATH>]
```

**示例（`/HR /F`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /hr /f
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/S`: 显示文件大小（字节）

**功能：** 显示文件字节数，可与 `/HR` 联用转换为人类可读格式。

**语法：**

```powershell
treepp (--size | -s | /S) [<PATH>]
```

**示例：**

```powershell
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/NI`: 不显示树形连接线

**功能：** 用纯空格缩进取代树形符号（每层 2 个空格）。

**语法：**

```powershell
treepp (--no-indent | -i | /NI) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /ni
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/R`: 逆序排序

**功能：** 将当前排序结果倒序输出。

**语法：**

```powershell
treepp (--reverse | -r | /R) [<PATH>]
```

**示例：**

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

### `/DT`: 显示最后修改日期

**功能：** 在条目后追加文件/目录的最后修改时间，格式为 `YYYY-MM-DD HH:MM:SS`（本地时区）。

**语法：**

```powershell
treepp (--date | -d | /DT) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /dt
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/X`: 排除匹配项

**功能：** 忽略与模式匹配的文件或目录。支持通配符 `*` 和 `?`。可多次指定以排除多个模式。

**语法：**

```powershell
treepp (--exclude | -I | /X) <PATTERN> [<PATH>]
```

**示例（排除 `*.md`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /x *.md
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

**示例（排除多个模式）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /x *.md /x LICENSE
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/L`: 限制递归深度

**功能：** 指定最大递归层级。`0` 表示仅显示根目录本身，`1` 表示根目录及其直接子项。

**语法：**

```powershell
treepp (--level | -L | /L) <LEVEL> [<PATH>]
```

**示例（仅展示 1 层）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /l 1
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/M`: 仅显示匹配项

**功能：** 只保留符合模式的文件条目（目录始终显示以保持结构）。支持通配符。可多次指定。

**语法：**

```powershell
treepp (--include | -m | /M) <PATTERN> [<PATH>]
```

**示例（仅显示 `*.rs`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /m *rs
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/DU`: 显示目录累计大小

**功能：** 统计每个目录的累计磁盘用量（递归计算所有子文件大小之和）。常与 `/HR` 配合使用。启用此选项会自动启用 `/S`。

> **注意：** 此选项需要批处理模式（`/B`），因为需要完整扫描树后才能计算累计大小。

**语法：**

```powershell
treepp (--disk-usage | -u | /DU) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /b /du /hr
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
D:.
└─src        378.5 KB
```

### `/RP`: 显示末尾统计信息

**功能：** 在输出末尾追加统计信息汇总，包括目录数、文件数（若启用 `/F`）和扫描耗时。

**语法：**

```powershell
treepp (--report | -e | /RP) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /rp
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/P`: 修剪空目录

**功能：** 隐藏不包含任何文件的目录节点（递归判断：仅含空子目录的目录也视为空）。

**语法：**

```powershell
treepp (--prune | -P | /P) [<PATH>]
```

**示例：**

假设存在空目录 `empty_dir`，使用 `/P` 后将不会显示：

```powershell
PS D:\数据\Rust\tree++> treepp /p /f
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

### `/NB`: 不显示 Windows 原生样板信息

**功能：** 省略 Windows 原生 `tree` 的卷信息和序列号输出（前两行）。

**语法：**

```powershell
treepp (--no-win-banner | -N | /NB) [<PATH>]
```

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /nb
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

> **性能提示：** 样板信息是通过在 `X:\__tree++__` 目录执行原生 `tree` 命令获取的。在性能敏感场景建议开启此选项。

### `/SI`: 终端静默

**功能：** 禁止向标准输出写入结果。

> **限制：** 必须与 `/O` 搭配使用，否则将报错。单独使用静默模式没有意义（无任何输出产生）。

**语法：**

```powershell
treepp (--silent | -l | /SI) [<PATH>]
```

**示例（`/F /O tree.txt /SI`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /o tree.txt /si
```

### `/O`: 输出到文件

**功能：** 将结果持久化到文件。支持的格式由扩展名决定，默认仍同时输出到控制台，可配合 `/SI` 静默。

**语法：**

```powershell
treepp (--output | -o | /O) <FILE> [<PATH>]
```

**支持的扩展名：**

| 扩展名            | 格式   | 是否需要 `/B` |
|----------------|------|-----------|
| `.txt`         | 纯文本  | 否         |
| `.json`        | JSON | 是         |
| `.yml` `.yaml` | YAML | 是         |
| `.toml`        | TOML | 是         |

> **注意：** 结构化输出格式（JSON/YAML/TOML）需要批处理模式（`/B`）。

**示例（TXT 格式，无需 `/B`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /f /o tree.txt
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

**示例（JSON 格式，需要 `/B`）：**

```powershell
PS D:\数据\Rust\tree++> treepp /b /f /o tree.json
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

### `/T`: 扫描线程数

**功能：** 指定扫描线程数量。值必须为正整数。

> **限制：** 此选项需要批处理模式（`/B`）。

**语法：**

```powershell
treepp (--thread | -t | /T) <N> [<PATH>]
```

**默认值：** 8

**示例：**

```powershell
PS D:\数据\Rust\tree++> treepp /b /f /t 16
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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


### `/G`: 遵循 `.gitignore`

**功能：** 解析每级目录中的 `.gitignore` 文件，自动忽略匹配条目。支持规则链继承：子目录继承父目录规则，同时应用自身规则。

**语法：**

```powershell
treepp (--gitignore | -g | /G) [<PATH>]
```

**示例：**

假设 `.gitignore` 中包含 `target/` 和 `*.log`，使用 `/G` 后这些条目将被忽略：

```powershell
PS D:\数据\Rust\tree++> treepp /f /g
卷 存储 的文件夹 PATH 列表
卷序列号为 26E9-52C1
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

## 参数限制汇总

| 参数    | 限制说明                                                        |
|-------|-------------------------------------------------------------|
| `/SI` | 必须与 `/O` 搭配使用                                               |
| `/T`  | 值必须为正整数（≥1），且需要 `/B`                                        |
| `/L`  | 值必须为非负整数（≥0）                                                |
| `/DU` | 需要 `/B`                                                     |
| `/O`  | 扩展名必须为 `.txt`、`.json`、`.yml`、`.yaml` 或 `.toml`；结构化格式需要 `/B` |

## 退出码

| 退出码 | 含义   |
|-----|------|
| 0   | 成功   |
| 1   | 参数错误 |
| 2   | 扫描错误 |
| 3   | 输出错误 |