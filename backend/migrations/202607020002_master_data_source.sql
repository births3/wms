-- Persist source for product, supplier and customer master data.

ALTER TABLE products
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'api_import';

ALTER TABLE suppliers
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'api_import';

ALTER TABLE customers
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'api_import';
