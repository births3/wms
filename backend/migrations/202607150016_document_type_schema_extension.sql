-- 同步后续业务单据类型已落地的受控参数，避免新增字典项使整个字典不可读。

UPDATE system_dictionary_categories
   SET param_schema = jsonb_set(
           jsonb_set(
               jsonb_set(
                   param_schema,
                   '{properties,direction,enum}',
                   '["inbound", "outbound", "internal"]'::jsonb
               ),
               '{properties,workflow_template,enum}',
               '["purchase_inbound", "sales_return", "other_inbound", "purchase_return_outbound", "sales_outbound", "sample_outbound", "other_outbound", "stock_loss", "stock_surplus", "quality_liaison"]'::jsonb
           ),
           '{properties,batch_policy,enum}',
           '["standard_batch", "specified_batch", "optional"]'::jsonb
       ),
       updated_at = now()
 WHERE dict_code = 'document_type';
