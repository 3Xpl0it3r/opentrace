# OpenTrace Dashboard - 待开发事项

## 后端 API 待添加

### Tracepoint/Tracer 扩展字段

当前 `tracepoints` 表缺少以下字段，需要添加到数据库 schema 和 Rust 模型中：

| 字段 | 类型 | 说明 |
|------|------|------|
| `events_sent` | INTEGER | 已发送事件数量 |
| `events_failed` | INTEGER | 失败事件数量 |

**涉及文件：**
- `src/db/schema.rs` - `CREATE_TABLES` 中的 `tracepoints` 表
- `src/db/tracepoints.rs` - `Tracepoint` 结构体
- `src/db/tracepoints.rs` - `get_tracepoint_by_id` / `list_tracepoints` 查询

### Tracepoint 更新 API

当前只有创建和删除，缺少更新接口：

```
PUT /api/agents/:agent_id/tracepoints/:tracepoint_id
Body: { "name": "tcp_connect", "description": "...", "enabled": true }
Response: Tracepoint
```

**涉及文件：**
- `src/handlers/tracepoints.rs` - 添加 `update_tracepoint` handler
- `src/lib.rs` - 注册路由

### Tracepoint 统计更新机制

需要一个机制来更新 `events_sent` 和 `events_failed`：
- 方案 A: Agent 定期上报统计到 Server
- 方案 B: Server 端统计（通过 sink 反向统计）
- 方案 C: Agent 直接更新数据库（如果 Agent 有 DB 访问权限）

---

## 前端待完善

### Tracer 编辑功能

AgentDetail.tsx 中编辑按钮目前是 `TODO`，需要：
- 添加编辑 Modal
- 调用 `PUT /api/agents/:id/tracepoints/:tp_id` 更新

### Dashboard 图表

当前图表是随机数据占位，需要接入真实数据：
- Throughput Overview - 需要时序数据 API
- Sink Distribution - 需要 sink 统计 API

---



### Agent 扩展字段

当前 `agents` 表缺少以下字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | TEXT | Agent 版本号 |
| `running_tracers` | INTEGER | 当前运行中的 tracer 数量 |
| `total_tracers` | INTEGER | 总共配置的 tracer 数量 |

**涉及文件：**
- `src/db/schema.rs` - `CREATE_TABLES` 中的 `agents` 表
- `src/models/agent.rs` - `Agent` 结构体
- `src/db/agents.rs` - 查询逻辑

### Agent Tracepoints 统计 API

需要一个 API 来获取每个 agent 的 tracepoints 统计（或在 list_agents 时一起返回）：

```
GET /api/agents/:id/tracepoints/stats
Response: { "running": 4, "total": 7 }
```

或者修改 `GET /api/agents` 返回时包含 `running_tracers` 和 `total_tracers` 字段。



### Tracer 运行配置字段

当前 `tracepoints` 表缺少以下配置字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `schedule` | TEXT | 运行时间配置：'always'（持续运行）或 crontab 表达式 |
| `sink_ids` | TEXT | 绑定的 Sink ID 列表（JSON 数组） |
| `metrics_config` | TEXT | Metrics 配置 JSON（endpoint, port, interval 等） |

**涉及文件：**
- `src/db/schema.rs` - `CREATE_TABLES` 中的 `tracepoints` 表
- `src/db/tracepoints.rs` - `Tracepoint` / `CreateTracepoint` 结构体
- `src/db/tracepoints.rs` - 更新逻辑

### Tracer Sink 绑定 API

需要为 Tracer 配置 Sink 绑定：

```
PUT /api/agents/:agent_id/tracepoints/:tp_id/sinks
Body: { "sink_ids": [1, 2, 3] }
Response: { "success": true }
```

或者复用现有的 Sink 绑定 API，在前端逐个调用。

*最后更新: 2026-06-19*
