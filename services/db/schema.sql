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
  counterparty text not null default '',
  event_type text not null,
  amount numeric(40,0),
  shares numeric(40,0),
  fee numeric(40,0),
  nav numeric(40,0),
  aum numeric(40,0),
  tx_hash text not null default '',
  ledger bigint not null default 0,
  raw jsonb not null default '{}'::jsonb,
  occurred_at timestamptz not null default now()
);

alter table deposit_withdraw_events
  add column if not exists counterparty text not null default '',
  add column if not exists fee numeric(40,0),
  add column if not exists nav numeric(40,0),
  add column if not exists aum numeric(40,0);

create table if not exists user_profiles (
  address text primary key,
  email text not null default '',
  x_handle text not null default '',
  display_name text not null default '',
  bio text not null default '',
  avatar_url text not null default '',
  notification_frequency text not null default 'off',
  drift_threshold_bps integer not null default 500,
  notification_email text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

alter table user_profiles
  add column if not exists display_name text not null default '',
  add column if not exists bio text not null default '',
  add column if not exists avatar_url text not null default '',
  add column if not exists notification_frequency text not null default 'off',
  add column if not exists drift_threshold_bps integer not null default 500,
  add column if not exists notification_email text not null default '';

create table if not exists profile_auth_challenges (
  nonce text primary key,
  address text not null,
  message text not null,
  expires_at timestamptz not null,
  used_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists profile_auth_challenges_address_idx
  on profile_auth_challenges (address, expires_at desc);

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

create index if not exists deposit_withdraw_events_account_idx
  on deposit_withdraw_events (account, counterparty, basket_id);
