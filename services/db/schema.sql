create table if not exists basket_configs (
  basket_id text primary key,
  creator text not null default '',
  name text not null default '',
  share_token_id text not null default '',
  assets jsonb not null default '[]'::jsonb,
  weights_bps jsonb not null default '[]'::jsonb,
  strategy jsonb not null default '{}'::jsonb,
  raw_config jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists rebalance_history (
  id bigserial primary key,
  basket_id text not null references basket_configs(basket_id) on delete cascade,
  tx_hash text not null default '',
  ledger bigint,
  old_weights_bps jsonb not null default '[]'::jsonb,
  new_weights_bps jsonb not null default '[]'::jsonb,
  drift_bps integer not null default 0,
  status text not null,
  created_at timestamptz not null default now()
);

create table if not exists deposit_withdraw_events (
  id bigserial primary key,
  event_id text not null unique,
  basket_id text not null,
  account text not null default '',
  event_type text not null,
  amount numeric(40,0),
  shares numeric(40,0),
  tx_hash text not null default '',
  ledger bigint not null default 0,
  raw jsonb not null default '{}'::jsonb,
  occurred_at timestamptz not null default now()
);

create table if not exists user_profiles (
  address text primary key,
  email text not null default '',
  x_handle text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists indexer_state (
  key text primary key,
  cursor text not null default '',
  last_ledger bigint not null default 0,
  updated_at timestamptz not null default now()
);

create index if not exists deposit_withdraw_events_basket_time_idx
  on deposit_withdraw_events (basket_id, occurred_at desc);

create index if not exists rebalance_history_basket_time_idx
  on rebalance_history (basket_id, created_at desc);
