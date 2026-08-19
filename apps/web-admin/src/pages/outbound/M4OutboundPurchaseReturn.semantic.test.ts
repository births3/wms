import type { PurchaseReturnOrder } from "./m4-outbound-page-model";

type AssertTrue<T extends true> = T;
type AssertFalse<T extends false> = T;

type HasPurchaseReturnBatchNo = "batch_no" extends keyof PurchaseReturnOrder ? true : false;
type HasPurchaseReturnDocumentType = "document_type" extends keyof PurchaseReturnOrder
  ? true
  : false;
type HasPurchaseReturnApprovalSource = "approval_source" extends keyof PurchaseReturnOrder
  ? true
  : false;

export type PurchaseReturnMustNotExposeBatchNo = AssertFalse<HasPurchaseReturnBatchNo>;
export type PurchaseReturnMustExposeDocumentType = AssertTrue<HasPurchaseReturnDocumentType>;
export type PurchaseReturnMustExposeApprovalSource =
  AssertTrue<HasPurchaseReturnApprovalSource>;
