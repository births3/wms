-- Add US-M1-010 compliance attributes and the 12-node default matrix.

UPDATE system_dictionary_categories
   SET param_schema = jsonb_build_object(
       'required', jsonb_build_array('requires_dual_sign'),
       'properties', COALESCE(param_schema -> 'properties', '{}'::jsonb) || jsonb_build_object(
           'requires_dual_person_matrix', jsonb_build_object('type', 'array'),
           'requires_dedicated_ledger', jsonb_build_object('type', 'boolean'),
           'requires_dedicated_storage', jsonb_build_object('type', 'boolean'),
           'requires_qualification', jsonb_build_object('type', 'array'),
           'regulation_basis', jsonb_build_object('type', 'string')
       )
   ),
       updated_at = now()
 WHERE dict_code = 'special_drug_category';

WITH matrix_nodes(ordinal, process_code, node_code) AS (
    VALUES
        (1, '入库', '收货'),
        (2, '入库', '验收'),
        (3, '入库', '上架'),
        (4, '出库', '拣货'),
        (5, '出库', '复核'),
        (6, '出库', '装箱'),
        (7, '出库', '发货交接'),
        (8, '报损', '报损执行'),
        (9, '报溢', '报溢执行'),
        (10, '销毁', '销毁执行'),
        (11, '退货', '退货验收'),
        (12, '退货', '退货上架')
), matrix_defaults AS (
    SELECT item.item_code,
           jsonb_agg(
               jsonb_build_object(
                   'process', node.process_code,
                   'node', node.node_code,
                   'policy', CASE
                       WHEN item.item_code IN ('narcotic', 'psychotropic_1', 'radioactive')
                           THEN 'dual_scan_with_approval'
                       WHEN item.item_code IN ('psychotropic_2', 'blood_product')
                           THEN 'dual_scan'
                       WHEN item.item_code = 'toxic_medical'
                            AND ((node.process_code = '入库' AND node.node_code = '验收')
                              OR (node.process_code = '出库' AND node.node_code = '复核')
                              OR (node.process_code = '报损' AND node.node_code = '报损执行')
                              OR (node.process_code = '销毁' AND node.node_code = '销毁执行'))
                           THEN 'dual_scan_with_approval'
                       WHEN item.item_code = 'toxic_medical'
                           THEN 'dual_scan'
                       WHEN item.item_code = 'vaccine'
                            AND ((node.process_code = '入库' AND node.node_code IN ('收货', '验收'))
                              OR (node.process_code = '出库' AND node.node_code = '复核')
                              OR (node.process_code = '报损' AND node.node_code = '报损执行'))
                           THEN 'dual_scan'
                       WHEN item.item_code = 'none'
                            AND ((node.process_code = '入库' AND node.node_code = '验收')
                              OR (node.process_code = '出库' AND node.node_code = '复核'))
                           THEN 'dual_scan'
                       ELSE 'single'
                   END
               ) ORDER BY node.ordinal
           ) AS matrix
      FROM system_dictionary_items item
      CROSS JOIN matrix_nodes node
     WHERE item.dict_code = 'special_drug_category'
       AND item.owner_id IS NULL
     GROUP BY item.item_code
)
UPDATE system_dictionary_items item
   SET params = item.params || jsonb_build_object(
       'requires_dual_person_matrix', matrix_defaults.matrix,
       'requires_dual_sign', item.item_code <> 'none',
       'requires_dedicated_ledger', item.item_code <> 'none',
       'requires_dedicated_storage', item.item_code IN ('narcotic', 'psychotropic_1', 'radioactive'),
       'requires_qualification', CASE
           WHEN item.item_code = 'none' THEN '[]'::jsonb
           ELSE jsonb_build_array('special_drug_business_license')
       END,
       'regulation_basis', CASE item.item_code
           WHEN 'narcotic' THEN '《麻醉药品和精神药品管理条例》'
           WHEN 'psychotropic_1' THEN '《麻醉药品和精神药品管理条例》'
           WHEN 'psychotropic_2' THEN '《麻醉药品和精神药品管理条例》'
           WHEN 'toxic_medical' THEN '《医疗用毒性药品管理办法》'
           WHEN 'radioactive' THEN '《放射性药品管理办法》'
           WHEN 'vaccine' THEN '《疫苗管理法》'
           WHEN 'blood_product' THEN '《血液制品管理条例》'
           ELSE ''
       END,
       'updated_at', now()
   )
  FROM matrix_defaults
 WHERE item.item_code = matrix_defaults.item_code
   AND item.dict_code = 'special_drug_category'
   AND item.owner_id IS NULL;
