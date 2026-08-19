-- M3 inventory status/color mapping must use the canonical quarantined status.

ALTER TABLE warehouse_zones
    DROP CONSTRAINT IF EXISTS warehouse_zones_quality_color_check;

ALTER TABLE warehouse_zones
    ADD CONSTRAINT warehouse_zones_quality_color_not_blank
    CHECK (btrim(quality_color) <> '');

UPDATE system_dictionary_categories
   SET param_schema = jsonb_set(
       param_schema,
       '{properties,inventory_quality_status,enum}',
       '["qualified", "quarantined", "unqualified"]'::jsonb,
       TRUE
   ),
       updated_at = now()
 WHERE dict_code = 'quality_color';

UPDATE system_dictionary_items
   SET params = jsonb_set(params, '{inventory_quality_status}', '"quarantined"'::jsonb, TRUE),
       updated_at = now(),
       version = version + 1
 WHERE dict_code = 'quality_color'
   AND item_code = 'quarantine_yellow'
   AND owner_id IS NULL;
