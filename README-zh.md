# `tree++`: 适用于Windows更好的`tree`命令

*[English](./README.md)*

Windows平台上的`tree`命令自从近40年前发布以来几乎就没有改动. 在如今LLM的时代, 作为描述项目结构非常常用的工具, 仅有的`/f`和`/a`两个参数的功能显然捉襟见肘. 同时, 它也不怎么快.  

`tree++`是对`tree`的一次全面升级, 开源于[GitHub](https://github.com/Water-Run/treepp). 包括:  

- 在兼容原有的Windows`tree`命令的基础上, 扩展指令集, 支持包括显示文件大小, 递归深度限制, 以及将结果输出至文件等常用功能  
- 支持传统Windows风格(如`/f`, 不区分大小写), Unix风格(如`-f`和`--files`)以及Powershell风格(如`-Files`)的指令
- 支持多线程, 在大且复杂的目录中, 轻松获取指数级的性能提升  

`tree++`使用`Rust`实现.  

## 安装

从[GitHub Release](https://github.com/Water-Run/treepp/releases/tag/1.0.0)下载`tree++.zip`, 解压到合适目录, 并将目录添加至环境变量.  

开启Windows终端, 执行:  

```powershell
treepp /v
```

有输出:  

```powershell
tree++ version 0.1.0
author: WaterRun
link: https://github.com/Water-Run/treepp
```

即完成安装.  

随后, 你可以和普通的Windows `tree`命令一样的使用:  

```powershell
treepp /f
```

## 速览

| 参数集(等价写法)                                           | 说明                                        |
| ----------------------------------------------------------- | ------------------------------------------- |
| `--help` `-h` `/?` `-Help`                                  | 显示帮助信息                                |
| `--version` `-v` `/V` `-Version`                            | 显示版本信息                                |
| `--ascii` `-a` `/A` `-Ascii`                                | 使用 ASCII 字符绘制树                       |
| `--files` `-f` `/F` `-Files`                                | 显示文件                                    |
| `--full-path` `-p` `/FP` `-FullPath`                        | 显示完整路径                                |
| `--human-readable` `-H` `/HR` `-HumanReadable`              | 以人类可读方式显示文件大小                  |
| `--no-indent` `-i` `/NI` `-NoIndent`                        | 不显示树形连接线                            |
| `--reverse` `-r` `/R` `-Reverse`                            | 逆序排序                                    |
| `--size` `-s` `/S` `-Size`                                  | 显示文件大小(字节)                          |
| `--date` `-d` `/DT` `-Date`                                 | 显示最后修改日期                            |
| `--exclude` `-I` `/X` `-Exclude`                            | 排除匹配的文件                              |
| `--level` `-L` `/L` `-Level`                                | 限制递归深度                                |
| `--include` `-m` `/M` `-Include`                            | 仅显示匹配的文件                            |
| `--quote` `-q` `/Q` `-Quote`                                | 用双引号包裹文件名                          |
| `--dirs-first` `-D` `/DF` `-DirsFirst`                      | 目录优先显示                                |
| `--disk-usage` `-u` `/DU` `-DiskUsage`                      | 显示目录累计大小                            |
| `--ignore-case` `-c` `/IC` `-IgnoreCase`                    | 匹配时忽略大小写                            |
| `--no-report` `-n` `/NR` `-NoReport`                        | 不显示末尾统计信息                          |
| `--prune` `-P` `/P` `-Prune`                                | 修剪空目录                                  |
| `--sort` `-S` `/SO` `-Sort`                                 | 指定排序方式(`name`、`size`、`mtime` 等)    |
| `--no-header` `-N` `/NH` `-NoHeader`                        | 不显示卷信息与头部报告信息                  |
| `--silent` `-l` `/SI` `-Silent`                             | 终端静默(结合`output`指令使用)                |
| `--output` `-o` `/O` `-Output`                              | 将结果输出至文件(`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T` `-Thread`                              | 扫描线程数(默认为24)                        |

> 完整参数集参阅: [tree++参数集文档](./OPTIONS-zh.md)
