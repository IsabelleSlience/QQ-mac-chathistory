# qq-mac-export-tools

一个面向新版 macOS QQ / NTQQ 本地数据库的研究型导出工具。

它做两件事：

- 读取 `nt_db` 目录中的加密数据库
- 按“每个联系人一个文件 / 每个群一个文件”的方式导出为适合 ChatGPT 结构化分析的 `CSV`

这个仓库**不包含你的真实数据库密钥**，也**不会自动替你提取密钥**。  
你需要先自行拿到运行时数据库 key，再把它作为参数传给工具。

## 当前能力

- 自动探测常见的 macOS QQ `nt_db` 目录
- 读取 `nt_msg.db`、`profile_info.db`
- 私聊文件命名优先用“备注”
- 如果没有备注，则使用“昵称__QQ号”
- 群聊文件命名优先用“群名__群号”
- 每个会话单独导出，尽量绕开局部坏页
- 生成 `direct_index.csv`、`group_index.csv`
- 如果某些群聊或私聊命中坏页，会记录到 `*_errors.csv`

## 已知限制

- 本仓库目前**不自动提取 Mac QQ 数据库 key**
- 如果数据库局部损坏，部分会话仍可能失败
- 当前更偏“研究/取证工具”，不是面向普通用户的一键 GUI 应用

## 环境

- macOS
- Rust stable

## 安装

```bash
git clone https://github.com/YOUR_NAME/qq-mac-export-tools.git
cd qq-mac-export-tools
cargo build --release
```

## 用法

### 1. 导出全部会话为 CSV

```bash
export QQ_DB_KEY='YOUR_16_BYTE_KEY'
cargo run --bin export_latest_csv -- \
  --key "$QQ_DB_KEY" \
  --db-root "/Users/you/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_xxx/nt_db" \
  --output "./exports"
```

如果不传 `--db-root`，程序会尝试自动探测最新的 `nt_db` 目录。

导出完成后，目录结构类似：

```text
exports/
  direct/
    Alice.csv
    Bob__123456789.csv
  group/
    Example_Group__541305724.csv
  direct_index.csv
  group_index.csv
  group_errors.csv
  README.txt
```

### 2. 检查单个私聊会话是否包含老记录

```bash
export QQ_DB_KEY='YOUR_16_BYTE_KEY'
cargo run --bin query_conversation -- \
  --key "$QQ_DB_KEY" \
  u_xxxxxxxxxxxxxxxxxxxxx
```

输出会显示：

- 消息总数
- 最早时间
- 最晚时间
- 前几条记录的发送者信息

## CSV 字段

每个会话文件都包含这些列：

- `conversation_id`
- `conversation_number`
- `conversation_label`
- `sender_uid`
- `sender_uin`
- `is_self`
- `send_time`
- `send_time_iso`
- `msg_type`
- `sub_msg_type`
- `send_status`
- `message_summary`

这套格式比 Excel 更适合：

- ChatGPT 上传分析
- Python / pandas 二次处理
- 批量搜索、筛选、按时间拆分

## 关于密钥提取

这部分单独写在：

- [docs/macos-key-extraction.md](./docs/macos-key-extraction.md)

## 适用范围

这套流程是在新版 Mac QQ / NTQQ 本地数据库上实测跑通的。  
不同 QQ 版本、不同数据库布局、不同损坏程度下，结果可能不同。

## 项目状态

- 当前更适合作为研究和数据导出工具
- 已经能覆盖很多私聊和部分群聊场景
- 针对损坏页面做了“按会话单独导出”的容错
- 未来还可以继续补：
  - 自动提取 key
  - 更稳定的群聊恢复
  - 更完整的消息类型解析

## 安全提醒

- 不要把你的真实 key 提交到 GitHub
- 不要把原始数据库直接公开上传
- 不要把含有私人聊天记录的导出结果直接推送到公共仓库
