/**
 * M1 主数据静态 seed（常量 + 商品/供应商/客户/仓库/库位 seed 与确定性生成器）。
 * 从 web-admin-dev-mock-model.ts 按职责拆出；仅 type-only 依赖 model（运行时依赖
 * 单向 model -> seeds，避免循环引用），model 通过 `export *` 保持全部导出不变。
 */
import type {
  DevCustomer,
  DevLocation,
  DevProduct,
  DevSupplier,
  DevWarehouse,
} from "./web-admin-dev-mock-model";

export const devOwnerId = "00000000-0000-0000-0000-000000000001";
export const devUserId = "00000000-0000-0000-0000-000000000101";
export const devWarehouseId = "00000000-0000-0000-0000-000000003001";
export const devLocationId = "00000000-0000-0000-0000-000000000201";
export const devLoginPassword = ["Correct", "Horse1!"].join("");

export let devProduct: DevProduct = {
  id: "00000000-0000-0000-0000-000000001001",
  owner_id: devOwnerId,
  product_code: "P-M1-001",
  product_name: "冷藏胰岛素注射液",
  spec: "10ml*1支",
  dosage_form: "注射剂",
  approval_no: "国药准字H20260001",
  manufacturer: "鹏鹞示例药业",
  special_drug_category_code: "none",
  udi_code: "06901234567891",
  electronic_regulatory_code: "81000000000000000001",
  length_mm: 120,
  width_mm: 100,
  height_mm: 30,
  volume_cm3: 360,
  weight_g: 180,
  packaging_levels: [
    {
      id: "00000000-0000-0000-0000-000000001011",
      unit_code: "piece",
      unit_name: "支",
      ratio_to_base: 1,
      is_base: true,
      is_default: false,
      sort_order: 1,
    },
    {
      id: "00000000-0000-0000-0000-000000001012",
      unit_code: "box",
      unit_name: "盒",
      ratio_to_base: 10,
      is_base: false,
      is_default: true,
      sort_order: 2,
    },
    {
      id: "00000000-0000-0000-0000-000000001013",
      unit_code: "case",
      unit_name: "箱",
      ratio_to_base: 200,
      is_base: false,
      is_default: false,
      sort_order: 3,
    },
  ],
  mapping_traces: [],
  attrs: {
    source: "api_import",
    storage_condition: "cold_2_8",
  } as Record<string, unknown>,
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

/** M1 商品档案 seed 目标条数：现有字面量 3 条（devProduct + P-M1-002/003）+ 生成器补齐至 100 */
const devSeedProductCount = 100;
const devSeedProductLiteralCount = 3;

/** 药品基组：[通用名, 规格, 剂型, 批准文号前缀 H=化学药 Z=中成药] */
const devSeedProductBases: Array<[string, string, string, string]> = [
  ["阿莫西林胶囊", "0.25g*24粒", "胶囊剂", "H"],
  ["头孢拉定胶囊", "0.25g*24粒", "胶囊剂", "H"],
  ["布洛芬缓释胶囊", "0.3g*20粒", "缓释胶囊剂", "H"],
  ["对乙酰氨基酚片", "0.5g*24片", "片剂", "H"],
  ["双氯芬酸钠肠溶片", "25mg*30片", "肠溶片剂", "H"],
  ["奥美拉唑肠溶胶囊", "20mg*14粒", "肠溶胶囊剂", "H"],
  ["雷贝拉唑钠肠溶片", "10mg*14片", "肠溶片剂", "H"],
  ["盐酸二甲双胍片", "0.5g*48片", "片剂", "H"],
  ["格列齐特缓释片", "30mg*30片", "缓释片剂", "H"],
  ["阿托伐他汀钙片", "10mg*14片", "片剂", "H"],
  ["瑞舒伐他汀钙片", "10mg*7片", "片剂", "H"],
  ["苯磺酸氨氯地平片", "5mg*28片", "片剂", "H"],
  ["缬沙坦胶囊", "80mg*7粒", "胶囊剂", "H"],
  ["厄贝沙坦片", "150mg*7片", "片剂", "H"],
  ["卡托普利片", "25mg*100片", "片剂", "H"],
  ["琥珀酸美托洛尔缓释片", "47.5mg*7片", "缓释片剂", "H"],
  ["氯雷他定片", "10mg*6片", "片剂", "H"],
  ["盐酸西替利嗪片", "10mg*12片", "片剂", "H"],
  ["左氧氟沙星片", "0.5g*6片", "片剂", "H"],
  ["阿奇霉素分散片", "0.25g*6片", "分散片剂", "H"],
  ["头孢克肟颗粒", "50mg*12袋", "颗粒剂", "H"],
  ["蒙脱石散", "3g*10袋", "散剂", "H"],
  ["健胃消食片", "0.8g*32片", "片剂", "Z"],
  ["藿香正气口服液", "10ml*10支", "口服液", "Z"],
  ["连花清瘟胶囊", "0.35g*24粒", "胶囊剂", "Z"],
  ["感冒灵颗粒", "10g*9袋", "颗粒剂", "Z"],
  ["板蓝根颗粒", "10g*20袋", "颗粒剂", "Z"],
  ["复方丹参滴丸", "27mg*180丸", "滴丸剂", "Z"],
  ["速效救心丸", "40mg*60丸", "滴丸剂", "Z"],
  ["六味地黄丸", "360丸/瓶", "浓缩丸剂", "Z"],
  ["逍遥丸", "200丸/瓶", "浓缩丸剂", "Z"],
  ["归脾丸", "200丸/瓶", "浓缩丸剂", "Z"],
  ["维C银翘片", "12片*2板", "片剂", "Z"],
  ["叶酸片", "0.4mg*31片", "片剂", "H"],
  ["碳酸钙D3片", "600mg*30片", "片剂", "H"],
  ["葡萄糖酸钙口服液", "10ml*10支", "口服液", "H"],
  ["维生素C片", "100mg*100片", "片剂", "H"],
  ["复合维生素B片", "100片/瓶", "片剂", "H"],
  ["维生素E软胶囊", "0.1g*60粒", "软胶囊剂", "H"],
  ["鱼肝油软胶囊", "30粒/瓶", "软胶囊剂", "H"],
  ["蒲地蓝消炎口服液", "10ml*10支", "口服液", "Z"],
  ["蓝芩口服液", "10ml*6支", "口服液", "Z"],
  ["蜜炼川贝枇杷膏", "150ml/瓶", "煎膏剂", "Z"],
  ["西瓜霜润喉片", "20片*2板", "片剂", "Z"],
  ["硝酸甘油片", "0.5mg*100片", "片剂", "H"],
  ["阿司匹林肠溶片", "100mg*30片", "肠溶片剂", "H"],
];

const devSeedManufacturers = [
  "华北制药",
  "哈药集团",
  "石药集团",
  "太极集团",
  "云南白药",
  "北京同仁堂",
  "修正药业",
  "葵花药业",
  "以岭药业",
  "华润三九",
  "扬子江药业",
  "正大天晴",
];

/** 生成第 N 个 seed 商品（N 从 1 起）：循环使用药品基组并补齐至 100 条。 */
function makeSeedProduct(seedIndex: number): DevProduct {
  const base = devSeedProductBases[(seedIndex - 1) % devSeedProductBases.length];
  const largePack = seedIndex > devSeedProductBases.length;
  // 大包装变体：装量数字翻倍（"0.25g*24粒" → "0.25g*48粒"）
  const spec = largePack
    ? base[1].replace(/(\d+)(粒|片|支|袋|丸)/, (_match, amount: string, unit: string) => `${Number(amount) * 2}${unit}`)
    : base[1];
  const index = devSeedProductLiteralCount + seedIndex;
  const manufacturer = devSeedManufacturers[(seedIndex - 1) % devSeedManufacturers.length];
  return {
    id: `00000000-0000-0000-0000-${String(1000 + index).padStart(12, "0")}`,
    owner_id: devOwnerId,
    product_code: `P-M1-${String(index).padStart(4, "0")}`,
    product_name: base[0],
    spec,
    dosage_form: base[2],
    approval_no: `国药准字${base[3]}2026${String(index).padStart(4, "0")}`,
    manufacturer,
    special_drug_category_code: "none",
    udi_code: `0690${String(10000000 + index).padStart(8, "0")}`,
    electronic_regulatory_code: `8100000000000000${String(index).padStart(4, "0")}`,
    length_mm: 100 + (seedIndex % 5) * 15,
    width_mm: 60 + (seedIndex % 4) * 10,
    height_mm: 30 + (seedIndex % 3) * 15,
    volume_cm3: 180 + (seedIndex % 9) * 40,
    weight_g: 50 + (seedIndex % 7) * 25,
    packaging_levels: [
      {
        id: `00000000-0000-0000-0000-${String(5000 + index).padStart(12, "0")}`,
        unit_code: "box",
        unit_name: "盒",
        ratio_to_base: 1,
        is_base: true,
        is_default: true,
        sort_order: 1,
      },
      {
        id: `00000000-0000-0000-0000-${String(6000 + index).padStart(12, "0")}`,
        unit_code: "case",
        unit_name: "箱",
        ratio_to_base: 40,
        is_base: false,
        is_default: false,
        sort_order: 2,
      },
    ],
    mapping_traces: [],
    attrs: {
      source: seedIndex % 4 === 0 ? "batch_import" : "api_import",
      storage_condition: seedIndex % 5 === 0 ? "cold_2_8" : "normal_10_30",
    } as Record<string, unknown>,
    status: seedIndex % 11 === 0 ? "disabled" : "active",
    created_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
    updated_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
  };
}

export const devSeedProducts: DevProduct[] = [
  devProduct,
  {
    id: "00000000-0000-0000-0000-000000001002",
    owner_id: devOwnerId,
    product_code: "P-M1-002",
    product_name: "批量导入感冒灵颗粒",
    spec: "10g*9袋",
    dosage_form: "颗粒剂",
    approval_no: "国药准字Z20260002",
    manufacturer: "鹏鹞示例药业",
    special_drug_category_code: "none",
    udi_code: "06901234567892",
    electronic_regulatory_code: "81000000000000000002",
    length_mm: 150,
    width_mm: 120,
    height_mm: 55,
    volume_cm3: 990,
    weight_g: 240,
    packaging_levels: [
      {
        id: "00000000-0000-0000-0000-000000001021",
        unit_code: "bag",
        unit_name: "袋",
        ratio_to_base: 1,
        is_base: true,
        is_default: false,
        sort_order: 1,
      },
      {
        id: "00000000-0000-0000-0000-000000001022",
        unit_code: "box",
        unit_name: "盒",
        ratio_to_base: 9,
        is_base: false,
        is_default: true,
        sort_order: 2,
      },
      {
        id: "00000000-0000-0000-0000-000000001023",
        unit_code: "case",
        unit_name: "箱",
        ratio_to_base: 216,
        is_base: false,
        is_default: false,
        sort_order: 3,
      },
    ],
    mapping_traces: [],
    attrs: {
      source: "batch_import",
      storage_condition: "normal_10_30",
    },
    status: "active",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  },
  {
    id: "00000000-0000-0000-0000-000000001003",
    owner_id: devOwnerId,
    product_code: "P-M1-003",
    product_name: "手工新建维生素片",
    spec: "100片/瓶",
    dosage_form: "片剂",
    approval_no: "国药准字H20260003",
    manufacturer: "鹏鹞示例药业",
    special_drug_category_code: "none",
    udi_code: "06901234567893",
    electronic_regulatory_code: "81000000000000000003",
    length_mm: 60,
    width_mm: 60,
    height_mm: 90,
    volume_cm3: 324,
    weight_g: 120,
    packaging_levels: [
      {
        id: "00000000-0000-0000-0000-000000001031",
        unit_code: "tablet",
        unit_name: "片",
        ratio_to_base: 1,
        is_base: true,
        is_default: false,
        sort_order: 1,
      },
      {
        id: "00000000-0000-0000-0000-000000001032",
        unit_code: "bottle",
        unit_name: "瓶",
        ratio_to_base: 100,
        is_base: false,
        is_default: true,
        sort_order: 2,
      },
      {
        id: "00000000-0000-0000-0000-000000001033",
        unit_code: "case",
        unit_name: "箱",
        ratio_to_base: 3000,
        is_base: false,
        is_default: false,
        sort_order: 3,
      },
    ],
    mapping_traces: [],
    attrs: {
      source: "manual",
      storage_condition: "normal_10_30",
    },
    status: "active",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  },
  ...Array.from({ length: devSeedProductCount - devSeedProductLiteralCount }, (_value, i) => makeSeedProduct(i + 1)),
];

export let devSupplier: DevSupplier = {
  id: "00000000-0000-0000-0000-000000001101",
  owner_id: devOwnerId,
  supplier_code: "S-M1-001",
  supplier_name: "鹏鹞示例供应商",
  license_no: "SPL-2026-001",
  contact_name: "王供应",
  source: "api_import",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devCustomer: DevCustomer = {
  id: "00000000-0000-0000-0000-000000001201",
  owner_id: devOwnerId,
  customer_code: "C-M1-001",
  customer_name: "鹏鹞示例门店",
  license_no: "CPL-2026-001",
  source: "api_import",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devWarehouse: DevWarehouse = {
  id: devWarehouseId,
  owner_id: devOwnerId,
  warehouse_code: "WH-M1-001",
  warehouse_name: "鹏鹞冷链仓",
  warehouse_type: "physical",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devLocation: DevLocation = {
  id: devLocationId,
  owner_id: devOwnerId,
  warehouse_id: devWarehouseId,
  zone_id: "00000000-0000-0000-0000-000000003101",
  location_code: "A01-01-02-03",
  row_no: 1,
  column_no: 2,
  layer_no: 3,
  max_volume_cm3: 1000000,
  used_volume_cm3: 120000,
  max_sku_count: 3,
  location_type: "storage",
  current_owner_id: null,
  status: "available",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

// —— M1 客商档案 seed 扩充：地区 × 业态确定性组合生成（无随机，重启一致）——
const devSeedRegionNames = [
  "北京", "上海", "广州", "深圳", "杭州", "南京", "苏州", "武汉", "成都", "重庆",
  "西安", "郑州", "长沙", "青岛", "大连", "天津", "福州", "昆明", "合肥", "石家庄",
  "宁波", "南昌", "无锡", "厦门", "济南",
];
const devSeedSupplierSuffixes = ["医药供应链", "药业", "医药贸易", "生物科技"];
const devSeedCustomerSuffixes = ["连锁大药房", "中医院", "人民医院", "医药零售"];
const devSeedContactFamilyNames = ["王", "李", "张", "刘", "陈", "杨", "赵", "黄", "周", "吴", "徐", "孙"];
const devSeedContactGivenNames = ["供应", "采购", "经理", "文华", "志强", "晓明", "丽华", "建国"];

/** 生成第 N 个 seed 供应商（N 从 1 起，共 100 个：25 地区 × 4 业态） */
function makeSeedSupplier(seedIndex: number): DevSupplier {
  const region = devSeedRegionNames[(seedIndex - 1) % devSeedRegionNames.length];
  const suffix = devSeedSupplierSuffixes[Math.floor((seedIndex - 1) / devSeedRegionNames.length)];
  const contactFamily = devSeedContactFamilyNames[(seedIndex - 1) % devSeedContactFamilyNames.length];
  const contactGiven = devSeedContactGivenNames[seedIndex % devSeedContactGivenNames.length];
  return {
    id: `00000000-0000-0000-0000-${String(2000 + seedIndex).padStart(12, "0")}`,
    owner_id: devOwnerId,
    supplier_code: `S-M1-${String(100 + seedIndex).padStart(4, "0")}`,
    supplier_name: `${region}鹏鹞${suffix}`,
    license_no: `SPL-2026-${String(100 + seedIndex).padStart(3, "0")}`,
    contact_name: `${contactFamily}${contactGiven}`,
    source: seedIndex % 4 === 0 ? "manual" : "api_import",
    status: seedIndex % 13 === 0 ? "disabled" : "active",
    created_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
    updated_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
  };
}

/** 生成第 N 个 seed 客户（N 从 1 起，共 100 个：25 地区 × 4 业态） */
function makeSeedCustomer(seedIndex: number): DevCustomer {
  const region = devSeedRegionNames[(seedIndex - 1) % devSeedRegionNames.length];
  const suffix = devSeedCustomerSuffixes[Math.floor((seedIndex - 1) / devSeedRegionNames.length)];
  return {
    id: `00000000-0000-0000-0000-${String(3000 + seedIndex).padStart(12, "0")}`,
    owner_id: devOwnerId,
    customer_code: `C-M1-${String(100 + seedIndex).padStart(4, "0")}`,
    customer_name: `${region}鹏鹞${suffix}`,
    license_no: `CPL-2026-${String(100 + seedIndex).padStart(3, "0")}`,
    source: seedIndex % 4 === 0 ? "manual" : "api_import",
    status: seedIndex % 13 === 0 ? "disabled" : "active",
    created_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
    updated_at: new Date(Date.UTC(2026, 5, 29 + (seedIndex % 25))).toISOString(),
  };
}

export const devSeedSuppliers: DevSupplier[] = Array.from({ length: 100 }, (_value, i) => makeSeedSupplier(i + 1));
export const devSeedCustomers: DevCustomer[] = Array.from({ length: 100 }, (_value, i) => makeSeedCustomer(i + 1));
