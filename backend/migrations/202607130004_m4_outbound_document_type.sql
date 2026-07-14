ALTER TABLE outbound_orders
    ADD COLUMN IF NOT EXISTS document_type TEXT;

UPDATE outbound_orders
   SET document_type = 'sales_outbound'
 WHERE document_type IS NULL OR btrim(document_type) = '';

ALTER TABLE outbound_orders
    ALTER COLUMN document_type SET DEFAULT 'sales_outbound',
    ALTER COLUMN document_type SET NOT NULL;

CREATE INDEX IF NOT EXISTS outbound_orders_owner_document_type_idx
    ON outbound_orders (owner_id, document_type, updated_at DESC);
