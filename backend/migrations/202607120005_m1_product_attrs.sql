ALTER TABLE products
    ADD COLUMN IF NOT EXISTS attrs JSONB NOT NULL DEFAULT '{}'::jsonb;

UPDATE products
   SET special_drug_category = 'none'
 WHERE special_drug_category = 'normal';

ALTER TABLE products
    ALTER COLUMN special_drug_category SET DEFAULT 'none';
