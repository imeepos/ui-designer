-- Rudder Server schema v1 (idempotent).
-- 用户 / 计费流水 / 生成记录 / 动态设置（价格、上游地址、密钥、JWT secret）。

CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    email         TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'user',      -- user | admin
    status        TEXT NOT NULL DEFAULT 'active',    -- active | disabled
    credits       BIGINT NOT NULL DEFAULT 0 CHECK (credits >= 0),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS generations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    kind            TEXT NOT NULL,                   -- generations | edits
    model           TEXT NOT NULL,
    n               INTEGER NOT NULL DEFAULT 1,
    cost            BIGINT NOT NULL DEFAULT 0,       -- 实扣积分（0 = 管理员豁免）
    status          TEXT NOT NULL DEFAULT 'running', -- running | ok | failed_refunded
    params          JSONB NOT NULL DEFAULT '{}',
    upstream_status INTEGER,
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transactions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users(id),
    kind           TEXT NOT NULL,                    -- signup_bonus | consume | refund | admin_adjust
    amount         BIGINT NOT NULL,                  -- 带符号变动
    balance_after  BIGINT NOT NULL,
    generation_id  UUID REFERENCES generations(id),
    note           TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_transactions_user_created ON transactions (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generations_user_created  ON generations (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generations_created       ON generations (created_at DESC);
