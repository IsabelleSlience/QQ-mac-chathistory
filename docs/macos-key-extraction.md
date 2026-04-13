# MacOS Key Extraction Notes

这份说明只记录**研究思路**，不包含任何真实用户密钥。

## 背景

新版 macOS QQ / NTQQ 的本地数据库通常位于：

```text
~/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_xxx/nt_db
```

核心消息库通常是：

- `nt_msg.db`
- `profile_info.db`

这些数据库不是普通明文 SQLite，通常需要运行时 key 才能解开。

## 已公开的方向

围绕 NTQQ 数据库的公开资料已经存在，但在 Mac 上通常还停留在：

- 手工分析 `wrapper.node`
- 定位 `nt_sqlite3_key_v2`
- 在运行时观察传给 SQLCipher 的 key

也就是说，**Mac 路线通常仍然需要手动研究运行时行为**。

## 一条可行的研究链路

下面这条链路是本项目整理过、可复验的思路：

1. 找到 QQ 的 `wrapper.node`
2. 确认数据库开库链路是否经过：
   - `nt_sqlite3_key_v2`
   - 或者更下游的 codec / SQLCipher 相关函数
3. 用调试器或动态插桩工具在运行时观察：
   - key 长度
   - key 原始字节
4. 再将得到的 key 用于只读解库

## 实操建议

- 尽量在**只读副本**上做数据库分析
- 保留原始 `nt_db` 目录，不要直接在原库上尝试修复
- 先验证单个会话的最早/最晚消息时间，再做整批导出
- 如果整表扫描会报 `database disk image is malformed`，优先尝试：
  - 每个会话单独导出
  - 记录失败会话而不是整批中断

## 风险与限制

- QQ 版本变化可能导致符号、调用链或偏移变化
- 数据库可能处于 `WAL` 活跃状态
- 某些库会出现局部坏页，导致整表扫描失败
- 这类研究天然要求较高的逆向与调试经验

## 本项目的边界

这个仓库目前：

- 提供**导出工具**
- 提供**研究说明**

但目前**不提供自动提取 key 的一键脚本**。  
原因是不同版本 QQ 的稳定性、风险和可维护性都还不够理想。
