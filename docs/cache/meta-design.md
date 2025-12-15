# Soffio 单站点单进程缓存设计方案（meta）

> 约束：单站点、单进程、无外部依赖（不引入 Redis 等）。

本方案在既定原则（公开站点缓存、可关闭、宽松即时一致性 + 严格最终一致性、无
TTL、0/低入侵、精准预热/失效、事件驱动、完整消费与最优消费计划、层级化消费、启动预热、观测闭环）基础上，结合 Soffio
缓存候选对象清单，给出可落地的单进程实现形态。

---

## 1. 目标与边界

### 1.1 缓存范围

- **仅缓存公开站点（Public）读路径**：公开页面（HTML）、公开 API（`/api/v1`）、RSS、Sitemap 等。
- **管理站点（Admin）不缓存**：所有管理查询实时读取。

### 1.2 可开关

- `cache.enabled`：全局启停；关闭时所有读路径直读并跳过事件消费。

### 1.3 一致性

- **写后尽快收敛**：写路径触发主动消费，尽量同步完成失效（保证即时一致性）。
- **最终一致性兜底**：自动消费窗口触发补偿性消费。

### 1.4 无 TTL

- 常态不依赖 TTL。
- 单进程重启会清空内存缓存，天然降低“永久脏缓存”的风险，但仍需容量上限控制（见第 9 节）。

---

## 2. 缓存层级与共享策略

### 2.1 L0：查询/对象缓存（主收益，Service 低入侵）

用于减少 DB 查询与聚合计算。

- **Singleton 缓存**（全局单例对象）
    - SiteSettings
    - Navigation（header/footer 等）
    - MonthCounts / TagCounts（sidebar 聚合）

- **KV 缓存（按 slug/id/prefix）**
    - Post：`slug -> Post`、`id -> Post`
    - Page：`slug -> Page`、`id -> Page`
    - ApiKey：`prefix -> ApiKey`

- **LRU 缓存（高基数列表，严格上限）**
    - 公共页面的分页列表（如首页/归档）
    - 仅缓存热点页（例如首页前 N 页），否则无 TTL 会导致 key 空间膨胀

### 2.2 L1：响应缓存（Web 层 0 入侵 middleware）

用于缓存组装成本高的最终响应。

- 优先做 L0；L1 作为增强：
    - HTML
    - RSS
    - sitemap.xml
    - 公开 API GET

### 2.3 共享策略

- **公开 API（`/api/v1`）与页面共享底层 L0 查询缓存**。

---

## 3. Cache Key 规范（单站点但维度显式化）

虽然单站点无需 `site_id`，但 key 必须显式纳入所有会改变输出语义的维度：

- `format`：Html / Json / Rss / Sitemap
- `route/path`（或业务命名）
- `params_hash`（query/cursor/filter 等）
- ...

建议定义统一枚举：

- `CacheKey::L0(...)`
- `CacheKey::L1Response(...)`

---

## 4. 反向索引 Registry（精准失效的关键）

单进程下建议实现 **双向索引**，支持精准失效与清理：

- `entity_to_keys: HashMap<EntityKey, HashSet<CacheKey>>`
- `key_to_entities: HashMap<CacheKey, HashSet<EntityKey>>`

### 4.1 EntityKey 设计

既要覆盖“单实体”，也要覆盖“派生集合/聚合对象”，以便把列表/聚合/feed/sitemap 纳入精准失效。

- `EntityKey::SiteSettings`
- `EntityKey::Navigation`
- `EntityKey::Post(id)`（加 `PostSlug(slug)`）
- `EntityKey::Page(id)`（加 `PageSlug(slug)`）
- `EntityKey::ApiKey(prefix)`
- **集合/派生**
    - `EntityKey::PostsIndex`（任何 post 变更影响的列表集合）
    - `EntityKey::PostAggTags`
    - `EntityKey::PostAggMonths`
    - `EntityKey::Feed`
    - `EntityKey::Sitemap`

### 4.2 Registry 写入时机

- 写入缓存时同步登记：`register(entity, key)`
- 删除缓存 key 时同步清理 registry（用 `key_to_entities` 反查清理）。

---

## 5. 依赖收集（让响应缓存也能精准失效）

L1需要在生成响应时记录依赖实体，否则只能粗粒度清空。

### 5.1 机制

- 使用 **task-local DependencyCollector**：
    - middleware 在请求开始创建 collector
    - service 在读取关键数据时 `deps.record(EntityKey)`
    - middleware 在响应写入 L1 时，将 `response_key` 绑定到 collector 收集的实体集合（写入 registry）

### 5.2 入侵性

- Web 层保持 0 入侵（middleware）
- Service 层低入侵：在关键读方法中增加 `record()`

---

## 6. 缓存事件体系（单进程内存队列）

### 6.1 事件发布

- Service 在 repo 持久化成功后发布事件。

### 6.2 队列实现

- 内存事件队列：`VecDeque<CacheEvent>` + `Mutex/RwLock`

### 6.3 至少一次语义下的要求

- **幂等性**：事件必须带 `idempotency_key`
- **乱序容忍**：引入 `epoch/version`（单调递增）
- **可合并性**：消费前合并去重，生成最优消费计划

### 6.4 事件类型（覆盖候选对象）

- `SiteSettingsUpdated`
- `NavigationUpdated`
- `PostUpserted { post_id, slug }`
- `PostDeleted { post_id, slug }`
- `PageUpserted { page_id, slug }`
- `PageDeleted { page_id, slug }`
- `ApiKeyUpserted { prefix }`
- `ApiKeyRevoked { prefix }`
- `WarmupOnStartup`

---

## 7. 消费策略：完整消费 + 最优计划 + 层级覆盖

### 7.1 触发方式

- **主动消费（即时一致性）**：写操作成功后统一由service层触发
- **自动消费（最终一致性兜底）**：超过窗口触发

### 7.2 完整消费与计划生成

- 每次消费：`drain(queue)` 拉取一批事件（可配置 batch limit 防止长阻塞）
- 计划生成：
    - 同实体多事件按 `epoch` 取最终态
    - 删除优先级高于更新
    - 多个 post 事件可合并为一次集合派生处理（PostsIndex/Agg/Feed/Sitemap）

### 7.3 执行顺序（先低层后高层）

1) **失效 L0**（对象/查询缓存）
2) **失效 L1**（响应缓存）
3) **预热（warm）**：优先 warm L0，再 warm L1

建议：失效同步执行（确保即时一致性），预热异步执行（降低写延迟）。

---

## 8. 写源矩阵（在线写入强制覆盖）

不包含离线迁移/导入导出；但在线写入必须全覆盖：

- 管理 UI 写
- 公开 API（`/api/v1`）写
- 后台作业写（发布/渲染/回滚等）

落地要求：

- 每一种写源都要：发布哪些事件、是否触发主动消费、失败重试/告警策略。
- 不依赖 HTTP middleware 触发消费，在 **service 写方法内**统一调用（这样job与http可以由同一路径触发）：
    - `publish_event()`
    - `consume_now_if_configured()`

---

## 9. 容量与安全边界（无 TTL 必须有）

无 TTL + 无外部依赖时，必须靠容量上限避免内存不可控：

- Singleton：无上限
- KV（Post/Page/ApiKey）：设置上限（按站点规模调）
- LRU（分页列表）：严格上限 + 仅缓存热点页
- L1（响应缓存）：严格上限 + 仅缓存公开 GET

安全注意：

- ApiKey 校验不建议做无限负缓存（随机 prefix 可撑爆内存）；如需负缓存必须极小上限。

---

## 10. 启动预热（最小化冷启动 miss）

启动时入队 `WarmupOnStartup`，消费时预热：

- SiteSettings
- Navigation
- TagCounts / MonthCounts
- 首页列表第一页（以及前 N 页）
- RSS
- Sitemap
- 首页返回的最新 N 篇 Post 详情
- Navigation 中可见的 Page 详情

---

## 11. 观测与运营（单进程最小闭环）

至少提供日志/metrics：

- L0：hit/miss/evict（按对象类型）
- L1：hit/miss/evict（按 format）
- Event：queue_len、drain_size、plan_actions、consume_ms、warm_ms、failures
- 写后一致性：write_triggered_consumes、auto_consume_runs、auto_consume_hit（兜底次数）

---

## 12. 推荐代码模块划分（便于 agent 具体化）

建议新增 `src/cache/`：

- `keys.rs`：`CacheKey` / `EntityKey` 定义
- `store.rs`：L0/L1 存储（singleton/KV/LRU）
- `registry.rs`：双向索引 registry
- `deps.rs`：DependencyCollector（task-local）与 `record()` API
- `events.rs`：事件定义、队列 publish/drain
- `planner.rs`：drain → merge → plan（失效集合、预热集合）
- `consumer.rs`：执行计划（invalidate/warm）
- `middleware.rs`：L1 response cache middleware（feature gate）

---

# 下一步设计重点工作（交给本地 agent 具体化的建议）

1) **候选对象 → 具体 key 与依赖映射表**

- 明确每个 service 读方法产生哪些 `EntityKey` 依赖，写入哪些 `CacheKey`。
- 明确 Post/Page 变更会影响哪些派生集合：列表、tag/month 聚合、RSS、Sitemap。

2) **事件—动作矩阵（最重要的落地件）**

- 对每一种写操作（管理 UI/API/job）：
    - 发布哪些事件
    - 消费计划应失效哪些 L0/L1 key
    - 是否预热哪些内容
    - 合并规则（同实体、多实体、delete 覆盖等）

3) **依赖收集的最小入侵插桩点**

- 在哪些 service 查询入口插入 `deps.record()` 才能覆盖所有公开页面与 API 输出。
- 确保 web/repo 仍 0 入侵。

4) **容量上限与热点策略**

- KV/LRU/L1 的上限与淘汰策略（LRU/clock 等）。
- 对分页列表、响应缓存，明确“热点页”定义与 key 计算方式。

5) **消费触发的统一化**

- 不靠 middleware而是靠service：后台作业写同样需要触发主动消费。
- 明确同步失效与异步预热的边界（避免写接口尾延迟过高）。

6) **观测落点**

- 结合现有日志/metrics 体系，把关键指标接入。

---

## 结论

该方案在单站点单进程无外部依赖的前提下，以“内存事件队列 + 双向 registry + 依赖收集 + 层级化最优消费计划”为核心，覆盖候选缓存对象并保留可扩展的维度化
key 设计。下一步最关键的工作是把事件类型、实体依赖与具体 cache key 的映射与合并规则在 codebase 层面具体化，并用观测指标验证“最小成本”目标。

