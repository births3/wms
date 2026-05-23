import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
  PageHeader,
  PrintPreview,
} from "@/components/business";
import { Printer, Download, Mail, FileText } from "lucide-react";

/**
 * M4Manifest — M4-005 随货同行单
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M4-005（GSP 法定随货同行单 / A4 模板 / 一货一单）
 * Wave：Wave 2.5（M4 出库）
 * 业务约束：必须含批号效期生产企业 + 双方盖章位 + 二维码追溯；冷链需附温度记录摘要
 *
 * @example
 *   <M4Manifest />
 */

interface ManifestItem {
  no: number;
  name: string;
  spec: string;
  manufacturer: string;
  batch: string;
  expiry: string;
  qty: string;
  unitPrice: string;
  totalPrice: string;
}

const ITEMS: ManifestItem[] = [
  { no: 1, name: "葡萄糖注射液", spec: "500ml × 24", manufacturer: "山东齐鲁制药",
    batch: "20250901A", expiry: "2027-09-01", qty: "5 件", unitPrice: "98.00", totalPrice: "490.00" },
  { no: 2, name: "葡萄糖注射液", spec: "500ml × 24", manufacturer: "山东齐鲁制药",
    batch: "20260301A", expiry: "2028-03-01", qty: "3 件", unitPrice: "98.00", totalPrice: "294.00" },
  { no: 3, name: "重组人胰岛素", spec: "3ml:300IU × 5", manufacturer: "甘李药业",
    batch: "20260315B", expiry: "2027-03-15", qty: "2 件", unitPrice: "486.00", totalPrice: "972.00" },
  { no: 4, name: "盐酸吗啡片", spec: "10mg × 100", manufacturer: "东北制药",
    batch: "20260101N", expiry: "2027-01-01", qty: "1 件", unitPrice: "320.00", totalPrice: "320.00" },
];

export function M4Manifest() {
  const [zoom, setZoom] = useState(0.55);

  const totalQty = ITEMS.reduce((s, i) => s + parseInt(i.qty), 0);
  const totalAmount = ITEMS.reduce((s, i) => s + parseFloat(i.totalPrice), 0).toFixed(2);

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="随货同行单"
        subtitle="M4-005 · 单号 SO-2026-0042 · GSP 法定文件 · A4 模板"
        actions={
          <>
            <Button size="sm" variant="ghost" onClick={() => (window.location.hash = "#m6-custom")} title="另存为自定义报表">
              ⇲ 另存为报表
            </Button>
            <Button variant="outline" size="sm">
              <Mail className="h-4 w-4 mr-1" /> 邮件发送
            </Button>
            <Button variant="outline" size="sm">
              <Download className="h-4 w-4 mr-1" /> 下载 PDF
            </Button>
            <Button size="sm">
              <Printer className="h-4 w-4 mr-1" /> 打印
            </Button>
          </>
        }
      />

      <div className="px-6 py-4 grid grid-cols-[1fr_320px] gap-4">
        {/* 打印预览 */}
        <PrintPreview
          template="a4"
          pageCount={1}
          currentPage={1}
          zoom={zoom}
          onZoomChange={setZoom}
        >
          {/* A4 内容 */}
          <div className="p-12 bg-white text-black font-sans" style={{ fontFamily: "SimSun, serif" }}>
            <div className="text-center mb-6">
              <h1 className="text-2xl font-bold mb-1">药品随货同行单</h1>
              <div className="text-xs text-gray-600">
                依据《药品经营质量管理规范》（GSP）第八十五条
              </div>
            </div>

            {/* 头部信息 */}
            <table className="w-full text-sm mb-4">
              <tbody>
                <tr>
                  <td className="border border-gray-400 px-3 py-1.5 bg-gray-50 w-[14%]">单号</td>
                  <td className="border border-gray-400 px-3 py-1.5 font-mono">SO-2026-0042</td>
                  <td className="border border-gray-400 px-3 py-1.5 bg-gray-50 w-[14%]">日期</td>
                  <td className="border border-gray-400 px-3 py-1.5">2026-05-22</td>
                </tr>
                <tr>
                  <td className="border border-gray-400 px-3 py-1.5 bg-gray-50">出库方</td>
                  <td className="border border-gray-400 px-3 py-1.5" colSpan={3}>
                    北京天竺仓 W001 · 许可证 GSP-BJ-20240301
                  </td>
                </tr>
                <tr>
                  <td className="border border-gray-400 px-3 py-1.5 bg-gray-50">收货方</td>
                  <td className="border border-gray-400 px-3 py-1.5" colSpan={3}>
                    北京同仁堂连锁药店 · 许可证 GSP-BJ-20240901 · 联系人 李女士 010-8888 9999
                  </td>
                </tr>
              </tbody>
            </table>

            {/* 商品明细 */}
            <table className="w-full text-xs mb-4 border border-gray-400">
              <thead>
                <tr className="bg-gray-100">
                  <th className="border border-gray-400 px-2 py-1.5 w-8">序</th>
                  <th className="border border-gray-400 px-2 py-1.5 text-left">品名</th>
                  <th className="border border-gray-400 px-2 py-1.5 text-left">规格</th>
                  <th className="border border-gray-400 px-2 py-1.5 text-left">生产企业</th>
                  <th className="border border-gray-400 px-2 py-1.5">批号</th>
                  <th className="border border-gray-400 px-2 py-1.5">效期</th>
                  <th className="border border-gray-400 px-2 py-1.5">数量</th>
                  <th className="border border-gray-400 px-2 py-1.5">单价</th>
                  <th className="border border-gray-400 px-2 py-1.5">金额</th>
                </tr>
              </thead>
              <tbody>
                {ITEMS.map((item) => (
                  <tr key={item.no}>
                    <td className="border border-gray-400 px-2 py-1.5 text-center">{item.no}</td>
                    <td className="border border-gray-400 px-2 py-1.5">{item.name}</td>
                    <td className="border border-gray-400 px-2 py-1.5">{item.spec}</td>
                    <td className="border border-gray-400 px-2 py-1.5">{item.manufacturer}</td>
                    <td className="border border-gray-400 px-2 py-1.5 font-mono">{item.batch}</td>
                    <td className="border border-gray-400 px-2 py-1.5 font-mono">{item.expiry}</td>
                    <td className="border border-gray-400 px-2 py-1.5 text-right">{item.qty}</td>
                    <td className="border border-gray-400 px-2 py-1.5 text-right">¥{item.unitPrice}</td>
                    <td className="border border-gray-400 px-2 py-1.5 text-right">¥{item.totalPrice}</td>
                  </tr>
                ))}
                <tr className="bg-gray-50 font-semibold">
                  <td colSpan={6} className="border border-gray-400 px-2 py-1.5 text-right">合计</td>
                  <td className="border border-gray-400 px-2 py-1.5 text-right">{totalQty} 件</td>
                  <td className="border border-gray-400 px-2 py-1.5"></td>
                  <td className="border border-gray-400 px-2 py-1.5 text-right">¥{totalAmount}</td>
                </tr>
              </tbody>
            </table>

            {/* 双方盖章 */}
            <div className="grid grid-cols-2 gap-8 mt-8">
              <div>
                <div className="text-sm mb-1">出库方质管员签字 / 单位盖章：</div>
                <div className="h-20 border-b border-gray-400 mb-2"></div>
                <div className="text-xs text-gray-600">日期：____________</div>
              </div>
              <div>
                <div className="text-sm mb-1">收货方验收员签字 / 单位盖章：</div>
                <div className="h-20 border-b border-gray-400 mb-2"></div>
                <div className="text-xs text-gray-600">日期：____________</div>
              </div>
            </div>

            {/* 底部追溯 */}
            <div className="mt-6 pt-4 border-t border-gray-400 flex items-center justify-between text-xs text-gray-600">
              <div>
                <div>追溯码二维码：</div>
                <div className="mt-1 w-16 h-16 bg-gray-300 inline-block flex items-center justify-center text-[10px]">
                  [QR]
                </div>
              </div>
              <div className="text-right">
                <div>· 本单一货一单 · 出库方留存联</div>
                <div>· 根据 GSP 第 85 条留存 5 年</div>
                <div>· 单号: SO-2026-0042 · 第 1 页 / 共 1 页</div>
              </div>
            </div>
          </div>
        </PrintPreview>

        {/* 右侧：打印参数 + 法规说明 */}
        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <FileText className="h-4 w-4" /> 打印设置
            </div>
            <div className="space-y-2 text-xs">
              <div className="flex justify-between">
                <span className="text-muted-foreground">模板</span>
                <span className="font-medium">A4 标准（210×297mm）</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">份数</span>
                <span>2 联（出库方 + 收货方）</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">字体</span>
                <span>SimSun 12pt</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">缩放</span>
                <span>{Math.round(zoom * 100)}%</span>
              </div>
            </div>
          </Card>

          <Card className="p-4">
            <div className="text-sm font-semibold mb-3">GSP 合规要点</div>
            <ul className="text-xs space-y-1.5 text-muted-foreground">
              <li>✓ 含批号 / 生产日期 / 效期</li>
              <li>✓ 含生产企业 / 经营企业</li>
              <li>✓ 双方盖章位预留</li>
              <li>✓ 含 GMP 二维码</li>
              <li>✓ 留存 5 年（电子+纸质）</li>
              <li>✓ 一货一单（不可合并多单）</li>
            </ul>
          </Card>

          <Card className="p-4 border-wms-warning/40 bg-wms-warning/5">
            <div className="text-xs">
              <div className="font-semibold text-wms-warning mb-1">冷链补充</div>
              <div className="text-muted-foreground">
                本单含胰岛素（冷链药品），自动附 24 小时温度记录摘要（独立于本单）
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
