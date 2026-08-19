import { Input, Label, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@wms/ui";

export const LOCK_OPTIONS = [
  { label: "隔离", value: "quarantine" },
  { label: "不合格", value: "rejected" },
];

export interface LockFieldsProps {
  lockCategory: string;
  onLockCategoryChange: (value: string) => void;
  reasonCode: string;
  onReasonCodeChange: (value: string) => void;
  witnessId: string;
  onWitnessIdChange: (value: string) => void;
  mqlId: string;
  onMqlIdChange: (value: string) => void;
  showCategory: boolean;
  showMql: boolean;
  createLiaison?: boolean;
  onCreateLiaisonChange?: (value: boolean) => void;
}

export function LockFields({
  lockCategory,
  onLockCategoryChange,
  reasonCode,
  onReasonCodeChange,
  witnessId,
  onWitnessIdChange,
  mqlId,
  onMqlIdChange,
  showCategory,
  showMql,
  createLiaison,
  onCreateLiaisonChange,
}: LockFieldsProps) {
  return (
    <div className="space-y-4">
      {showCategory ? (
        <div className="space-y-2">
          <Label htmlFor="lpn-lock-category">锁类别</Label>
          <Select value={lockCategory} onValueChange={onLockCategoryChange}>
            <SelectTrigger id="lpn-lock-category" aria-label="锁类别">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LOCK_OPTIONS.map((item) => (
                <SelectItem key={item.value} value={item.value}>
                  {item.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : null}
      {showCategory ? (
        <div className="space-y-2">
          <Label htmlFor="lpn-lock-reason">原因字典码</Label>
          <Input
            id="lpn-lock-reason"
            value={reasonCode}
            onChange={(event) => onReasonCodeChange(event.target.value)}
            aria-label="原因字典码"
          />
        </div>
      ) : null}
      <div className="space-y-2">
        <Label htmlFor="lpn-lock-witness">见证人用户 ID</Label>
        <Input
          id="lpn-lock-witness"
          value={witnessId}
          onChange={(event) => onWitnessIdChange(event.target.value)}
          aria-label="见证人用户 ID"
        />
      </div>
      {showMql ? (
        <div className="space-y-2">
          <Label htmlFor="lpn-lock-mql">M-QL 单 ID</Label>
          <Input
            id="lpn-lock-mql"
            value={mqlId}
            onChange={(event) => onMqlIdChange(event.target.value)}
            placeholder={showCategory ? "不合格必填；可勾选下方自动建单" : "可选"}
            aria-label="M-QL 单 ID"
          />
        </div>
      ) : null}
      {showCategory && onCreateLiaisonChange ? (
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={Boolean(createLiaison)}
            onChange={(event) => onCreateLiaisonChange(event.target.checked)}
            aria-label="创建联系单并加锁"
          />
          创建联系单并加锁
        </label>
      ) : null}
    </div>
  );
}
