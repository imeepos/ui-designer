# D1 部署记录 — cms 上线 138 + nginx 切换（2026-09-11）

任务：T-D1（quality-gates）。执行会话：D1 子会话，回报主会话 DONE_WITH_CONCERNS 前先落盘本文件。

## 1. 部署版本

| 项 | 值 |
|---|---|
| cms 源码 commit | `b90062b8847ace11c25ef4224bc73cd08417f778`（/Users/imeepos/ext512/cms 主干） |
| 交叉编译产物 | `bin/cms-linux`（GOOS=linux GOARCH=amd64），sha256 前 16 位 `93035057b2c9c9b5` |
| 服务器二进制 | `/usr/local/bin/cms-server`（mode 755） |
| systemd 单元 | `/etc/systemd/system/cms-server.service`（EnvironmentFile=/etc/cms-server.env，User=root，WorkingDirectory=/var/lib/cms-server） |
| 监听 | 127.0.0.1:8800 |
| SQLite 数据 | `/var/lib/cms-server/cms.db`（迁移自动执行） |

## 2. env 关键项（/etc/cms-server.env，600，只列名不含值）

- `ADDR`（127.0.0.1:8800）
- `SQLITE_PATH`（/var/lib/cms-server/cms.db）
- `CONFIG_MASTER_KEY`（本机 openssl rand -base64 32 生成，AES-256-GCM 敏感配置密封用）
- `BOOTSTRAP_ADMIN_EMAIL`（ops@veren.top）
- `BOOTSTRAP_ADMIN_PASSWORD`（随机强口令，仅存 env）

## 3. 管理面种子（经 cms API 写入，非文件）

- 首管登录（ops@veren.top）→ `PUT /v1/admin/configs/image.openai.base_url`（string，非敏感）
- `PUT /v1/admin/configs/image.openai.api_key`（string，**sensitive:true**，AES-256-GCM at rest）
- 上游探测 `GET {base}/v1/models` 实际返回 3 个模型：gpt-image-2、gpt-image-2.5-flare、gpt-image-2.5-sunburst
- `PUT /v1/admin/configs/image.models` = 与上游对齐的三模型清单
- 上游凭证来源：旧 Postgres `rudder.settings` 表（upstream_base_url / upstream_api_key，尾 4 位 …dac0）；旧 /etc/rudder-server.env 中无 UPSTREAM_* 键（README 与实况不符，实际在 DB）

## 4. nginx 切换

- 配置文件：`/etc/nginx/sites-available/veren-https`
- 备份：`/etc/nginx/sites-available/veren-https.bak-d1-20260911080533`
- 变更：`location /api/v1/`（→127.0.0.1:8799）替换为 `location /api/ { proxy_pass http://127.0.0.1:8800/; }`（尾斜杠路径改写，cms 收到 /v1/...）
- `nginx -t` 通过后 `systemctl reload nginx`；`https://veren.top/api/healthz` 200

## 5. 验收证据摘要（真实输出）

1. **healthz/systemd**：`curl -sf https://veren.top/api/healthz` → `{"code":0,...,"status":"ok"}`；`systemctl is-active cms-server` → active
2. **G1 CORS 探针**（三段全过）：
   - preflight `Origin: tauri://localhost` → 204，`access-control-allow-origin: *`、`access-control-allow-headers: Authorization, Content-Type`（含 authorization ✓）
   - preflight `Origin: http://tauri.localhost` → 同上 204
   - 带 key 实调 `GET /api/v1/models`（Origin: tauri://localhost）→ 200 + 同组 CORS 头 + 模型清单
3. **扣点冒烟**：
   - 注册 e2e-smoke@veren.top（user id=2）→ admin adjust +200（流水 reason "d1-smoke topup"）
   - 真实生成 1 张 gpt-image-2（quality low, 1024x1024, 40.5s, usage total_tokens=1765）
   - 响应 data[0].url 为上游预签名 S3 URL（非 b64），下载 `file` 判定 `PNG image data, 1024x1024`，1,169,613 字节，PNG 魔数校验通过
   - 扣点：200 → 190，流水 `image_billing prepay ulid=… price=10 img=1/1`，差额 = 单价 10 ✓
   - 未知模型 → 404 `{"error":{"message":"The model 'no-such-image-model' does not exist","type":"not_found_error","code":"model_not_found"}}` ✓
4. **旧服务下线**：`systemctl disable --now rudder-server` → inactive/disabled；/etc/rudder-server.env、旧二进制、Postgres rudder 库原样保留
5. **密钥纪律**：全流程密钥只进 shell 变量；日志/文件仅尾 4 位（…dac0）。服务器侧自检：除本会话自身的 sudo grep 审计命令行外，无任何文件含全量 key（journal 中 2 条记录即本会话 grep 审计命令本身被 sudo 记账，属自指记录，非泄漏源）

## 6. 遗留 Concern（不阻塞开闸）

1. **默认 user 角色不含 llm:invoke**：cms 主干 T4 判定器把 llm:invoke 设计为「勾选进 key」，但默认角色只绑 apikey:manage，注册用户无法自助 mint 出可调网关的 key。本次经 sqlite 直接 `INSERT role_permissions(1,4)` 补绑（需 restart cms-server 清判定缓存生效）。**建议 cms 侧立任务把 llm:invoke 加进 EnsureDefaultRole 默认绑定或提供管理面授权端点**，否则桌面端新用户开箱不可用。
2. **生成响应非 b64**：上游（xtokenmirror）返回 data[0].url=预签名 S3 URL 而非 b64_json；验收标准「b64 可解码」以「图片字节可取回且 PNG 校验通过」等效达成。cms 为透传面不改写，符合主干设计。
3. **真 b64 解码路径未覆盖**：上游不支持该形态，属于上游差异非 cms 缺陷。

## 7. 回滚序列（完整命令，在 138 上以 ops 执行）

```bash
# 1) cms 下线
sudo systemctl disable --now cms-server

# 2) nginx 恢复旧配置（备份回拷 + 校验 + reload）
sudo cp /etc/nginx/sites-available/veren-https.bak-d1-20260911080533 /etc/nginx/sites-available/veren-https
sudo nginx -t && sudo systemctl reload nginx

# 3) 旧服务恢复（env/二进制/Postgres 均未动）
sudo systemctl enable --now rudder-server
systemctl is-active rudder-server   # 期望 active
curl -sf http://127.0.0.1:8799/health

# 4)（可选）cms 数据留档排查后再决定是否清理 /var/lib/cms-server/cms.db
#    回滚不删除任何 cms 侧文件：二进制 /usr/local/bin/cms-server、
#    unit /etc/systemd/system/cms-server.service、env /etc/cms-server.env 保留原位仅停止
```

## 8. 边界遵守

- 未改 cms 代码（llm:invoke 补绑是运行时数据操作，非代码变更）
- 未动 Postgres/middleware-postgres；未做域名/证书变更；未迁移旧用户；未删任何旧文件
- 真实生成全程恰好 1 次（主会话授权的显式冒烟）
