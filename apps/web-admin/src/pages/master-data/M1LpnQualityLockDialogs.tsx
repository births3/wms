import * as React from "react";
import { FormDialogTemplate } from "@wms/ui";

import {
  useApplyLpnQualityLockMutation,
  useChangeLpnQualityLockMutation,
  useReleaseLpnQualityLockMutation,
  type LpnContainer,
} from "@/features/master-data/lpn-container-queries";
import { BUTTON_SAVE } from "@/lib/ui-strings";
import { LockFields } from "./M1LpnQualityLockFields";

export interface LpnQualityLockDialogsProps {
  selected: LpnContainer | null;
  lockOpen: boolean;
  changeOpen: boolean;
  releaseOpen: boolean;
  onLockOpenChange: (open: boolean) => void;
  onChangeOpenChange: (open: boolean) => void;
  onReleaseOpenChange: (open: boolean) => void;
}

export function LpnQualityLockDialogs({
  selected,
  lockOpen,
  changeOpen,
  releaseOpen,
  onLockOpenChange,
  onChangeOpenChange,
  onReleaseOpenChange,
}: LpnQualityLockDialogsProps) {
  const applyLock = useApplyLpnQualityLockMutation();
  const changeLock = useChangeLpnQualityLockMutation();
  const releaseLock = useReleaseLpnQualityLockMutation();
  const [lockCategory, setLockCategory] = React.useState("quarantine");
  const [reasonCode, setReasonCode] = React.useState("");
  const [witnessId, setWitnessId] = React.useState("");
  const [mqlId, setMqlId] = React.useState("");
  const [createLiaison, setCreateLiaison] = React.useState(false);
  const [reviewMessage, setReviewMessage] = React.useState<string | null>(null);

  function resetForm() {
    setLockCategory("quarantine");
    setReasonCode("");
    setWitnessId("");
    setMqlId("");
    setCreateLiaison(false);
    setReviewMessage(null);
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
          const liaisonId = mqlId.trim() || null;
          void applyLock
            .mutateAsync({
              id: selected.id,
              body: {
                lock_category: lockCategory,
                reason_dict_item_code: reasonCode.trim(),
                witness_id: witnessId.trim(),
                quality_liaison_id: liaisonId,
                create_liaison: createLiaison && !liaisonId,
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
          createLiaison={createLiaison}
          onCreateLiaisonChange={setCreateLiaison}
        />
      </FormDialogTemplate>
      <FormDialogTemplate
        open={changeOpen}
        onOpenChange={(open) => {
          if (open) {
            resetForm();
            if (selected?.current_lock_category === "rejected") {
              setLockCategory("rejected");
            }
          }
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
                quality_liaison_id: mqlId.trim() || null,
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
          showMql
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
        errorMessage={releaseLock.error?.message ?? reviewMessage}
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
            .then((result) => {
              const skipped = result.skipped_batches ?? [];
              if (skipped.length > 0) {
                setReviewMessage(
                  `已解锁。${skipped.length} 条批次未回写（他流程已改状态），请人工复核。`,
                );
                return;
              }
              onReleaseOpenChange(false);
            });
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
