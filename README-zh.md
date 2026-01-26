# `tree++`: 好的多的Windows`tree`命令

*[English](./README.md)*

Windows上的`tree`命令自从近40年前发布以来几乎就没有改动. 在如今LLM的时代, 作为描述项目结构非常常用的工具, 仅有的`/f`和`/a`两个参数的功能显然捉襟见肘. 同时, 它也不太快.  

**`tree++`是对`tree`的一次全面升级**, 为Windows平台下的`tree`命令引入了:  

- ***扩展参数集, 支持功能涵盖包括显示文件大小, 递归深度限制, 修改输出风格, 将结果输出至文件, 及排除指定目录(包括遵循`.gitignore`)等常用功能***  
- ***Rust的实现更好的性能, 在批处理模式下更支持多线程, 提供显著的扫描速度提升***
- ***与原有的Windows`tree`命令参数和输出格式达到diff级别的完全兼容, 并可使用Unix风格的参数(如`-f`和`--files`)***

**`tree++`使用`Rust`实现**, 开源于[GitHub](https://github.com/Water-Run/treepp).  

*性能对比(以`C:\Windows`为示例):*  

| 类型                         |  耗时 (`ms`) |      倍率 |
|----------------------------|-----------:|--------:|
| `tree /f` (Windows Native) | `20721.90` | `1.00x` |
| `treepp /f`                |  `7467.99` | `2.77x` |
| `treepp /f /nb`            |  `7392.34` | `2.80x` |
| `treepp /f /nb /b`         |  `3226.38` | `6.42x` |
| `treepp /f /nb /b /t 1`    |  `9123.00` | `2.27x` |
| `treepp /f /nb /b /t 2`    |  `5767.71` | `3.59x` |
| `treepp /f /nb /b /t 4`    |  `3948.73` | `5.25x` |
| `treepp /f /nb /b /t 8`    |  `3166.81` | `6.54x` |
| `treepp /f /nb /b /t 16`   |  `2704.67` | `7.66x` |

## 安装

从[Release](https://github.com/Water-Run/treepp/releases)下载`tree++.zip`, 解压到合适目录, 并将目录添加至环境变量.  

开启Windows终端, 执行:  

```powershell
treepp /v
```

有输出:  

```plaintext
tree++ version 0.3.0

A Much Better Windows tree Command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

即完成安装.  

随后, 你可以以和普通的Windows `tree`命令一样的方式使用:  

```powershell
treepp /f
```

## 速览

| 参数集(等价写法)                     | 说明                                         |
|-------------------------------|--------------------------------------------|
| `--help` `-h` `/?`            | 显示帮助信息                                     |
| `--version` `-v` `/V`         | 显示版本信息                                     |
| `--ascii` `-a` `/A`           | 使用 ASCII 字符绘制树                             |
| `--files` `-f` `/F`           | 显示文件                                       |
| `--full-path` `-p` `/FP`      | 显示完整路径                                     |
| `--human-readable` `-H` `/HR` | 以人类可读方式显示文件大小                              |
| `--no-indent` `-i` `/NI`      | 不显示树形连接线                                   |
| `--reverse` `-r` `/R`         | 逆序排序                                       |
| `--all` `-k` `/AL`            | 显示隐藏文件                                     |
| `--size` `-s` `/S`            | 显示文件大小(字节)                                 |
| `--date` `-d` `/DT`           | 显示最后修改日期                                   |
| `--exclude` `-I` `/X`         | 排除匹配的文件                                    |
| `--level` `-L` `/L`           | 限制递归深度                                     |
| `--include` `-m` `/M`         | 仅显示匹配的文件                                   |
| `--disk-usage` `-u` `/DU`     | 显示目录累计大小                                   |
| `--report` `-e` `/RP`         | 显示末尾统计信息                                   |
| `--no-win-banner` `-N` `/NB`  | 不显示 Windows 原生 tree 的样板信息                  |
| `--silent` `-l` `/SI`         | 终端静默(结合`output`指令使用)                       |
| `--output` `-o` `/O`          | 将结果输出至文件(`.txt`, `.json`, `.yml`, `.toml`) |
| `--batch` `-b` `/B`           | 使用批处理模式                                    |
| `--thread` `-t` `/T`          | 扫描线程数(批处理模式, 默认8线程)                        |
| `--gitignore` `-g` `/G`       | 遵循`.gitignore`                             |

> 完整参数集参阅: [tree++参数集文档](./OPTIONS-zh.md)
