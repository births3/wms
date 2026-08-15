import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  Input,
} from "@wms/ui";
import { Search } from "lucide-react";

import type { MasterDataRow } from "@/features/master-data/master-data-queries";

interface ProductLookupProps {
  batchNo: string;
  products: readonly MasterDataRow[];
  onSelect: (product: MasterDataRow) => void;
}

interface ProductLookupFieldProps extends ProductLookupProps {
  errorMessage?: string;
  loading: boolean;
  placeholder?: string;
  required?: boolean;
  value: string;
  onChange: (value: string) => void;
  onOpenLookup: () => void;
}

interface ProductLookupDialogProps extends ProductLookupProps {
  open: boolean;
  query: string;
  onOpenChange: (open: boolean) => void;
}

export function ProductLookupField({
  batchNo,
  errorMessage,
  loading,
  placeholder,
  products,
  required = false,
  value,
  onChange,
  onOpenLookup,
  onSelect,
}: ProductLookupFieldProps) {
  const [suggestionsOpen, setSuggestionsOpen] = React.useState(false);
  const suggestions = React.useMemo(
    () => filterAsnProductLookupRows(products, value).slice(0, 6),
    [products, value],
  );
  const showSuggestions = suggestionsOpen && value.trim().length > 0;

  return (
    <div className="relative grid gap-1 text-xs text-muted-foreground">
      <span>ASN 商品编码</span>
      <div className="flex gap-2">
        <Input
          aria-label="ASN 商品编码"
          placeholder={placeholder}
          required={required}
          value={value}
          onBlur={() => setSuggestionsOpen(false)}
          onChange={(event) => {
            onChange(event.target.value);
            setSuggestionsOpen(true);
          }}
          onDoubleClick={onOpenLookup}
          onFocus={() => setSuggestionsOpen(true)}
        />
        <Button type="button" variant="outline" size="icon" onClick={onOpenLookup} title="关联商品档案">
          <Search className="size-4" aria-hidden />
          <span className="sr-only">关联商品档案</span>
        </Button>
      </div>
      {showSuggestions && (
        <div className="absolute left-0 right-11 top-full z-50 mt-1 max-h-64 overflow-auto rounded-md border bg-background p-2 shadow-lg">
          <LookupHeader />
          {suggestions.length > 0 ? (
            <ProductLookupRows batchNo={batchNo} products={suggestions} onSelect={onSelect} />
          ) : (
            <div className="px-2 py-3 text-xs text-muted-foreground">未匹配到商品档案</div>
          )}
        </div>
      )}
      {loading && <span className="text-xs text-muted-foreground">正在读取商品档案</span>}
      {errorMessage && <span className="text-xs text-destructive">{errorMessage}</span>}
    </div>
  );
}

export function ProductLookupDialog({
  batchNo,
  open,
  products,
  query,
  onOpenChange,
  onSelect,
}: ProductLookupDialogProps) {
  const filteredProducts = filterAsnProductLookupRows(products, query);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>关联商品档案</DialogTitle>
          <DialogDescription>选择商品档案后回填 ASN 商品编码。</DialogDescription>
        </DialogHeader>
        <div className="rounded-md border p-2">
          <LookupHeader />
          {filteredProducts.length > 0 ? (
            <ProductLookupRows batchNo={batchNo} products={filteredProducts} onSelect={onSelect} />
          ) : (
            <div className="py-8 text-center text-sm text-muted-foreground">
              未匹配到商品档案{query.trim() ? `：${query.trim()}` : ""}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ProductLookupRows({ batchNo, products, onSelect }: ProductLookupProps) {
  return products.map((product) => (
    <button
      key={product.id}
      type="button"
      className="grid w-full grid-cols-[1fr_1.2fr_1fr_0.8fr] gap-2 rounded px-2 py-2 text-left text-xs text-foreground hover:bg-accent"
      onMouseDown={(event) => {
        event.preventDefault();
        onSelect(product);
      }}
    >
      <span className="font-medium">{product.code}</span>
      <span>{product.name}</span>
      <span>{product.primaryValue}</span>
      <span>{batchText(batchNo)}</span>
    </button>
  ));
}

function LookupHeader() {
  return (
    <div className="grid grid-cols-[1fr_1.2fr_1fr_0.8fr] gap-2 px-2 pb-1 text-[11px] font-medium text-muted-foreground">
      <span>商品编码</span>
      <span>商品名称</span>
      <span>规格</span>
      <span>批号</span>
    </div>
  );
}

export function filterAsnProductLookupRows(
  products: readonly MasterDataRow[],
  keyword: string,
): MasterDataRow[] {
  const query = keyword.trim().toLocaleLowerCase("zh-CN");
  const rows = query
    ? products.filter((product) =>
        [product.code, product.name, product.primaryValue].some((value) =>
          value.toLocaleLowerCase("zh-CN").includes(query),
        ),
      )
    : products;
  return rows.slice(0, 20);
}

function batchText(batchNo: string) {
  return batchNo.trim() || "-";
}
