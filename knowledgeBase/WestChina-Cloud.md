- 数据源怎么来：`DSUtils.addDs(Source)`（`westChina-common/westChina-common-datasource/src/main/java/com/westChina/common/datasource/utils/DSUtils.java:52-76`）运行期 `DynamicRoutingDataSource.addDataSource(slave, ds)`；新租户建库由 `initSlaveDatabase`（`DSUtils.java:147-152`）自动完成。
- 变更怎么同步：`DSUtils.syncDs`（`:113-117`）发 MQ → `SourceSynchronization.execute`（`westChina-common/westChina-common-datasource/src/main/java/com/westChina/common/datasource/synchronization/SourceSynchronization.java:58`）按 `SyncType`（ADD/REFRESH/DELETE）刷新本地路由表。
- 文件隔离：MinIO **一租户一桶**，桶名由登录用户的企业名解析（`westChina-modules/westChina-ct/src/main/java/com/westChina/ct/controller/ctUploadController.java` 的 `getBucketName`）。
**为什么不用共享表 + tenant_id**：医疗数据敏感 + 客户要求"数据不混在一起"（合规与心理接受度），物理隔离能给出"数据在同库同表但带标记"给不了的背书；代价是连接池数量 × 租户数、schema 变更要全租户同步、跨租户统计困难。

(Showing lines 1199-1202 of 1674. Use offset=1203 to continue.)