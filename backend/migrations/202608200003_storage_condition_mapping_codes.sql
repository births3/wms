-- Phase 1 五温区收敛后，M-PM 储存条件映射目标值仍停留在 normal/cold/cool/frozen。
-- 与 products_storage_condition_check / validate_product_storage_condition 对齐。
-- 只改全局字典（owner_id IS NULL）。货主覆盖若仍映射旧四码，写入商品会被 CHECK 拒绝，需货主自行改规则。

UPDATE parameter_mapping_dictionaries
   SET target_values = '["normal_10_30", "cool_le_20", "cold_2_8", "freeze_le_minus_20", "ultra_cold_minus_80"]'::jsonb,
       updated_at = now()
 WHERE dict_code = 'storage_condition'
   AND owner_id IS NULL;

UPDATE parameter_mapping_rules AS rule
   SET target_value = CASE rule.target_value
           WHEN 'normal' THEN 'normal_10_30'
           WHEN 'cool' THEN 'cool_le_20'
           WHEN 'cold' THEN 'cold_2_8'
           WHEN 'frozen' THEN 'freeze_le_minus_20'
           ELSE rule.target_value
       END,
       updated_at = now()
  FROM parameter_mapping_dictionaries AS dictionary
 WHERE dictionary.id = rule.dictionary_id
   AND dictionary.dict_code = 'storage_condition'
   AND dictionary.owner_id IS NULL
   AND rule.target_value IN ('normal', 'cool', 'cold', 'frozen');
