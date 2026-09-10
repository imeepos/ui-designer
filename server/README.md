# rudder-server

舵 Rudder 的 Go 后端：用户系统 + 按次计费 + gpt-image-2 上游代理。
客户端（桌面端/CLI）**0 配置**：不再自备 `baseUrl/apiKey/model`，注册登录即用；
上游地址与密钥由管理员在后台配置，永不下发。

## 部署形态

```
客户端 ──HTTPS── nginx(veren.top, /api/v1/) ──127.0.0.1:8799── rudder-server ──上游 gpt-image-2 服务
                                                   │
                                             Postgres (docker: middleware-postgres, 库 rudder)
```

## 配置（环境变量，见 /etc/rudder-server.env）

| 变量 | 说明 | 默认 |
|---|---|---|
| `DATABASE_URL` | Postgres 连接串（必填） | — |
| `LISTEN_ADDR` | 监听地址（仅环回，nginx 反代） | `127.0.0.1:8799` |
| `ADMIN_USERNAME` / `ADMIN_PASSWORD` | 内置管理员；密码缺省时首次启动随机生成并打印一次 | `admin` / — |
| `UPSTREAM_BASE_URL` / `UPSTREAM_API_KEY` / `UPSTREAM_MODEL` | 首次启动种子值；之后以管理台设置为准 | — / — / `gpt-image-2` |
| `GEN_TIMEOUT_SEC` | 上游单次尝试超时 | `300` |

## API（全部挂在 `/api/v1` 下）

**公开**：`POST /auth/register`、`POST /auth/login`、`GET /health`

**登录用户**（`Authorization: Bearer <token>`）：

- `GET /auth/me` — 当前用户 + 积分
- `POST /auth/change-password`
- `POST /images/generations` — OpenAI 兼容 JSON；`model` 由服务端注入
- `POST /images/edits` — multipart，`image[]` 参考图（锚点在前），文本字段同上
- `GET /models` — 连通性探测（免费）
- `GET /usage` — 余额 + 最近流水/生成记录

**管理员**：`GET /admin/stats`、`GET /admin/users`、`PATCH /admin/users/{id}`（status/role）、
`POST /admin/users/{id}/credits`（amount 正充负扣）、`POST /admin/users/{id}/reset-password`、
`GET|PUT /admin/settings`（价格/注册开关/上游地址/密钥——密钥只回尾 4 位）、
`GET /admin/generations`、`GET /admin/console`（**Web 管理控制台**）

## 计费规则

- 每张图片 `creditsPerImage` 积分（默认 **10**，管理台可改）；`n` 张按倍数扣。
- 生成前**预扣**（事务 + 行锁，不会扣成负数）；上游失败**全额退还**并记录流水。
- **admin 角色生图免计费**（成本 0，仍记录生成）。
- 新用户注册赠送 `signupBonusCredits`（默认 0，可配）。
- `PUT /admin/settings` 可关注册、改上游地址/模型/密钥；密钥只存 Postgres `settings` 表，
  界面与 API 只显示尾 4 位，绝不回传全量、绝不写日志。

## 本地开发

```bash
go test ./...            # 单元测试
./deploy.sh              # 交叉编译并部署到 138（需 ssh 免密 + 已初始化的 env 文件）
```

## 服务器初始化（一次性，已执行过的无需重复）

```bash
# 1. 建库（复用 middleware-postgres 实例）
ssh ops@43.240.223.138 'docker exec middleware-postgres psql -U app -d appdb -c "CREATE DATABASE rudder OWNER app"'
# 2. env 文件（管理员密码自定）
ssh ops@43.240.223.138 'sudo tee /etc/rudder-server.env >/dev/null <<EOF
DATABASE_URL=postgres://app:<密码>@127.0.0.1:5432/rudder?sslmode=disable
LISTEN_ADDR=127.0.0.1:8799
ADMIN_USERNAME=admin
ADMIN_PASSWORD=<管理员密码>
EOF
sudo chmod 600 /etc/rudder-server.env'
# 3. systemd
scp server/systemd/rudder-server.service ops@43.240.223.138:/tmp/
ssh ops@43.240.223.138 'sudo mv /tmp/rudder-server.service /etc/systemd/system/ && sudo systemctl daemon-reload'
# 4. nginx：veren-https 的 server 块内追加
#    location /api/v1/ { proxy_pass http://127.0.0.1:8799; proxy_read_timeout 330s;
#      proxy_set_header Host $host; proxy_set_header X-Real-IP $remote_addr; client_max_body_size 32m; }
# 5. ./deploy.sh
```
