# Research Workflow

这份文档描述的是一条**可复用的研究流程**，目标是：

1. 定位新版 Mac QQ / NTQQ 的本地数据库
2. 判断数据库是否需要运行时 key
3. 围绕 `wrapper.node` 做静态和动态分析
4. 拿到 key 之后只读验证数据库
5. 再做分会话导出

这不是“一键自动提取 secret”的成品说明，而是一条可公开、可维护、可复验的研究路线。

## 1. 确认数据位置

先确认你分析的确实是当前活跃账号的数据目录。

常见路径：

```text
~/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_xxx/nt_db
```

重点看：

- `nt_msg.db`
- `profile_info.db`
- `group_info.db`
- `recent_contact.db`
- `*-wal`
- `*-shm`

## 2. 判断数据库状态

在开始任何逆向前，先确认当前问题属于哪一类：

- 根本打不开：说明 key 或数据库布局还没确认
- 可以打开部分：说明路径和 key 可能对，但存在坏页或活跃 WAL
- 单个会话能读、整表扫会崩：说明应该改成“按会话逐个导出”

建议先做只读副本。

## 3. 静态分析 wrapper.node

这个仓库提供：

```bash
cargo run --bin inspect_wrapper -- --wrapper /path/to/wrapper.node --verbose
```

它不会替你提取 key，但会帮助你确认：

- 是否还能看到 `nt_sqlite3_key_v2`
- 是否存在 `sqlcipher` / `codec` / `set_pass` 类标记
- 是否还能看到和 `nt_msg.db`、消息表有关的字符串

静态分析目标不是“拿到 key”，而是**缩小动态分析入口**。

## 4. 动态分析目标

重点不是盲目 hook 全部密码学函数，而是优先围绕：

- `nt_sqlite3_key_v2`
- codec / set pass 相关函数
- 开库时机

建议把问题拆成三个更稳定的小目标：

1. 找到开库链路是否真的经过某个私有入口
2. 确认 key 长度
3. 确认某次命中的 key 是否真能打开 `nt_msg.db`

## 5. 验证 key

一旦拿到候选 key，不要先整库导出。  
先做这两个验证：

1. 能否只读打开 `nt_msg.db`
2. 单个熟悉会话的最早/最晚时间是否符合预期

仓库里提供：

```bash
cargo run --bin query_conversation -- --key 'YOUR_KEY' u_xxx
```

这能帮你快速确认：

- 某个私聊会话是否真的存在
- 最早消息时间是否已经覆盖目标年份

## 6. 导出策略

优先使用：

- 每个联系人一个文件
- 每个群一个文件
- 单个会话单独扫描

不要默认整表一次性扫完。  
原因是局部坏页会让全量导出直接失败。

推荐导出格式：

- `CSV`

理由：

- 适合 ChatGPT 上传分析
- 适合 pandas / sqlite / shell 批量处理
- 比 Excel 更轻更通用

## 7. 命名策略

建议私聊文件命名：

1. 备注
2. 如果没有备注，则昵称 + QQ 号
3. 再不行就退回稳定 UID

这样后续做：

- 搜索
- 去重
- 上传给模型

都会明显更顺手。

## 8. 边界

这套研究流程适合公开，因为它强调的是：

- 方法
- 验证
- 只读分析
- 容错导出

而不是把自动提取运行时 secret 的脚本直接商品化或一键化。

如果要继续工程化，最值得做的是：

- 更好的坏页恢复
- 更完整的群聊导出
- 更强的消息类型摘要
- 更稳定的 wrapper 静态分析辅助
