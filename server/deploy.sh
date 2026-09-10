#!/usr/bin/env bash
# 部署 rudder-server 到 138 服务器（交叉编译 linux/amd64 → 上传 → 重启）。
# 用法: server/deploy.sh
# 前提: 本机 ssh 免密可达 ops@43.240.223.138；目标机已有 /etc/rudder-server.env。
set -euo pipefail

SSH="${LINUX_SSH_138:-ops@43.240.223.138}"
APP_DIR="/opt/rudder-server"
SERVICE="rudder-server"

cd "$(dirname "$0")"

echo "==> 交叉编译 linux/amd64"
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags "-s -w" -o bin/rudder-server .

echo "==> 上传二进制"
ssh "$SSH" "sudo mkdir -p $APP_DIR && sudo chown $(id -un) $APP_DIR"
scp bin/rudder-server "$SSH:/tmp/rudder-server.new"

echo "==> 安装并重启服务"
ssh "$SSH" "sudo mv /tmp/rudder-server.new $APP_DIR/rudder-server && sudo chmod 755 $APP_DIR/rudder-server && sudo systemctl restart $SERVICE && sleep 1 && systemctl is-active $SERVICE"

echo "==> 健康检查"
ssh "$SSH" "curl -fsS http://127.0.0.1:8799/api/v1/health"
echo
echo "部署完成。"
