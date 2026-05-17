-- V001__create_accounts.sql
-- Multi-tenant isolation: every resource belongs to an account.
-- Accounts are the top-level tenant boundary.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE accounts (
    account_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name TEXT NOT NULL,
    plan_tier    TEXT NOT NULL DEFAULT 'free'
                    CHECK (plan_tier IN ('free', 'pro', 'enterprise')),
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
