ALTER TABLE customers
    ADD COLUMN IF NOT EXISTS contact_name TEXT,
    ADD COLUMN IF NOT EXISTS contact_phone TEXT,
    ADD COLUMN IF NOT EXISTS business_scope TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS qualification_certificates JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS chain_name TEXT;

CREATE INDEX IF NOT EXISTS customers_owner_type_idx
    ON customers (owner_id, customer_type, updated_at DESC);
