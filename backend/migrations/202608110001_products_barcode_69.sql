-- 69 码（中国商品条码）字段。
ALTER TABLE products ADD COLUMN IF NOT EXISTS barcode_69 TEXT;
