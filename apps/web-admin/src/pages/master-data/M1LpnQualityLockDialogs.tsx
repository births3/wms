import * as React from "react";
import { FormDialogTemplate, Input, Label, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@wms/ui";

import {
  useApplyLpnQualityLockMutation,
  useChangeLpnQualityLockMutation,
  useReleaseLpnQualityLockMutation,
  type LpnContainer,
} from "@/features/master-data/lpn-container-queries";
import { BUTTON_SAVE } from "@/lib/ui-strings";

const LOCK_OPTIONS = [
  { label: "隔离", value: "quarantine" },
  { label: "不合格", value: "rejected" },
];

export function LpnQualityLockDialogs({
  selected,
  lockOpen,
  changeOpen,
  releaseOpen,
  onLockOpenChange,
  onChangeOpenChange,
  onReleaseOpenChange,
}: {
  selected: LpnContainer | null;
  lockOpen: boolean;
  changeOpen: boolean;
  releaseOpen: boolean;
  onLockOpenChange: (open: boolean) => void;
  onChangeOpenChange: (open: boolean) => void;
  onReleaseOpenChange: (open: boolean) => void;
}) {
  const applyLock = useApplyLpnQualityLockMutation();
  const changeLock = useChangeLpnQualityLockMutation();
  const releaseLock = useReleaseLpnQualityLockMutation();
  const [lockCategory, setLockCategory] = React.useState("quarantine");
  const [reasonCode, setReasonCode] = React.useState("");
  const [witnessId, setWitnessId] = React.useState("");
  const [mqlId, setMqlId] = React.useState("");

  function resetForm() {
    setLockCategory("quarantine");
    setReasonCode("");
    setWitnessId("");
    setMqlId("");
  }

  return (
    <>
      <FormDialogTemplate
        open={lockOpen}
        onOpenChange={(open) => {
          if (open) resetForm();
          onLockOpenChange(open);
        }}
        title="加锁"
        description={selected ? `LPN ${selected.lpn_code}，加锁需双人见证。不合格必须挂接 M-QL。` : "请先选择容器。"}
        submitLabel={BUTTON_SAVE}
        loading={applyLock.isPending}
        errorMessage={applyLock.error?.message}
        onSubmit={(event) => {
          event.preventDefault();
          if (!selected) return;
          void applyLock
            .mutateAsync({
              id: selected.id,
              body: {
                lock_category: lockCategory,
                reason_dict_item_code: reasonCode.trim(),
                witness_id: witnessId.trim(),
                quality_liaison_id: mqlId.trim() || null,
              },
            })
            .then(() => onLockOpenChange(false));
        }}
      >
        <LockFields
          lockCategory={lockCategory}
          onLockCategoryChange={setLockCategory}
          reasonCode={reasonCode}
          onReasonCodeChange={setReasonCode}
          witnessId={witnessId}
          onWitnessIdChange={setWitnessId}
          mqlId={mqlId}
          onMqlIdChange={setMqlId}
          showCategory
          showMql
        />
      </FormDialogTemplate>
      <FormDialogTemplate
        open={changeOpen}
        onOpenChange={(open) => {
          if (open) resetForm();
          onChangeOpenChange(open);
        }}
        title="换原因"
        description={selected ? `LPN ${selected.lpn_code}` : "请先选择容器。"}
        submitLabel={BUTTON_SAVE}
        loading={changeLock.isPending}
        errorMessage={changeLock.error?.message}
        onSubmit={(event) => {
          event.preventDefault();
          if (!selected) return;
          void changeLock
            .mutateAsync({
              id: selected.id,
              body: {
                lock_category: lockCategory,
                reason_dict_item_code: reasonCode.trim(),
                witness_id: witnessId.trim(),
              },
            })
            .then(() => onChangeOpenChange(false));
        }}
      >
        <LockFields
          lockCategory={lockCategory}
          onLockCategoryChange={setLockCategory}
          reasonCode={reasonCode}
          onReasonCodeChange={setReasonCode}
          witnessId={witnessId}
          onWitnessIdChange={setWitnessId}
          mqlId={mqlId}
          onMqlIdChange={setMqlId}
          showCategory
          showMql={false}
        />
      </FormDialogTemplate>
      <FormDialogTemplate
        open={releaseOpen}
        onOpenChange={(open) => {
          if (open) resetForm();
          onReleaseOpenChange(open);
        }}
        title="解锁"
        description={selected ? `LPN ${selected.lpn_code}，解锁需双人见证，且 M-QL 须已办结。` : "请先选择容器。"}
        submitLabel={BUTTON_SAVE}
        loading={releaseLock.isPending}
        errorMessage={releaseLock.error?.message}
        onSubmit={(event) => {
          event.preventDefault();
          if (!selected) return;
          void releaseLock
            .mutateAsync({
              id: selected.id,
              body: {
                witness_id: witnessId.trim(),
                quality_liaison_id: mqlId.trim() || null,
              },
            })
            .then(() => onReleaseOpenChange(false));
        }}
      >
        <LockFields
          lockCategory={lockCategory}
          onLockCategoryChange={setLockCategory}
          reasonCode={reasonCode}
          onReasonCodeChange={setReasonCode}
          witnessId={witnessId}
          onWitnessIdChange={setWitnessId}
          mqlId={mqlId}
          onMqlIdChange={setMqlId}
          showCategory={false}
          showMql
        />
      </FormDialogTemplate>
    </>
  );
}

function LockFields({
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
}: {
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
}) {
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
            placeholder={showCategory ? "不合格必填" : "可选"}
            aria-label="M-QL 单 ID"
          />
        </div>
      ) : null}
    </div>
  );
}
