//! Wave 3 M9 billing account and contract model service.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingChargeCalculation, BillingContract, BillingRule,
    CalculateBillingChargesRequest, CreateBillingAccountRequest, CreateBillingContractRequest,
    CreateBillingRuleRequest,
};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BillingError {
    NotFound,
    DuplicateAccountCode(String),
    DuplicateContractNo(String),
    InvalidRate,
    InvalidQuantity,
    InvalidEffectiveWindow,
    BillingRuleConflict,
}

#[derive(Clone, Debug, Default)]
pub struct BillingStore {
    accounts: BTreeMap<Uuid, BillingAccount>,
    contracts: BTreeMap<Uuid, BillingContract>,
    rules: BTreeMap<Uuid, BillingRule>,
}

impl BillingStore {
    pub fn create_account(
        &mut self,
        ctx: &AuthContext,
        req: CreateBillingAccountRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingAccount, BillingError> {
        if self.accounts.values().any(|account| {
            account.owner_id == ctx.owner_id && account.account_code == req.account_code
        }) {
            return Err(BillingError::DuplicateAccountCode(req.account_code));
        }
        let account = BillingAccount {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            account_code: req.account_code,
            account_name: req.account_name,
            status: "active".to_string(),
            created_at: now,
        };
        self.accounts.insert(account.id, account.clone());
        Ok(account)
    }

    pub fn create_contract(
        &mut self,
        ctx: &AuthContext,
        req: CreateBillingContractRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingContract, BillingError> {
        let account = self
            .accounts
            .get(&req.account_id)
            .filter(|account| account.owner_id == ctx.owner_id)
            .ok_or(BillingError::NotFound)?;
        if self.contracts.values().any(|contract| {
            contract.owner_id == ctx.owner_id && contract.contract_no == req.contract_no
        }) {
            return Err(BillingError::DuplicateContractNo(req.contract_no));
        }
        let valid_from = parse_date(&req.valid_from)?;
        let valid_to = parse_date(&req.valid_to)?;
        if valid_to < valid_from {
            return Err(BillingError::InvalidEffectiveWindow);
        }
        let contract = BillingContract {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            account_id: account.id,
            contract_no: req.contract_no,
            valid_from: valid_from.to_string(),
            valid_to: valid_to.to_string(),
            status: "active".to_string(),
            created_at: now,
        };
        self.contracts.insert(contract.id, contract.clone());
        Ok(contract)
    }

    pub fn create_rule(
        &mut self,
        ctx: &AuthContext,
        req: CreateBillingRuleRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingRule, BillingError> {
        if req.unit_price_cents < 0 {
            return Err(BillingError::InvalidRate);
        }
        let contract = self
            .contracts
            .get(&req.contract_id)
            .filter(|contract| contract.owner_id == ctx.owner_id)
            .ok_or(BillingError::NotFound)?;
        let effective_from = parse_date(&req.effective_from)?;
        let effective_to = parse_date(&req.effective_to)?;
        if effective_to < effective_from {
            return Err(BillingError::InvalidEffectiveWindow);
        }
        for existing in self.rules.values() {
            if existing.owner_id != ctx.owner_id
                || existing.contract_id != contract.id
                || existing.charge_item != req.charge_item
                || existing.unit != req.unit
                || existing.billing_cycle != req.billing_cycle
            {
                continue;
            }
            let existing_from = parse_date(&existing.effective_from)?;
            let existing_to = parse_date(&existing.effective_to)?;
            if existing_from <= effective_to && existing_to >= effective_from {
                return Err(BillingError::BillingRuleConflict);
            }
        }
        let rule = BillingRule {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            contract_id: contract.id,
            charge_item: req.charge_item,
            unit: req.unit,
            unit_price_cents: req.unit_price_cents,
            billing_cycle: req.billing_cycle,
            effective_from: effective_from.to_string(),
            effective_to: effective_to.to_string(),
            created_at: now,
        };
        self.rules.insert(rule.id, rule.clone());
        Ok(rule)
    }

    pub fn list_accounts(&self, ctx: &AuthContext) -> Vec<BillingAccount> {
        self.accounts
            .values()
            .filter(|account| account.owner_id == ctx.owner_id)
            .cloned()
            .collect()
    }

    pub fn calculate_period_charges(
        &self,
        ctx: &AuthContext,
        req: CalculateBillingChargesRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingChargeCalculation, BillingError> {
        if req.quantity < 0 {
            return Err(BillingError::InvalidQuantity);
        }
        let period_start = parse_date(&req.period_start)?;
        let period_end = parse_date(&req.period_end)?;
        if period_end < period_start {
            return Err(BillingError::InvalidEffectiveWindow);
        }
        let contract = self
            .contracts
            .get(&req.contract_id)
            .filter(|contract| contract.owner_id == ctx.owner_id)
            .ok_or(BillingError::NotFound)?;
        let rule = self
            .rules
            .values()
            .filter(|rule| {
                if rule.owner_id != ctx.owner_id
                    || rule.contract_id != contract.id
                    || rule.charge_item != req.charge_item
                {
                    return false;
                }
                let Ok(effective_from) = parse_date(&rule.effective_from) else {
                    return false;
                };
                let Ok(effective_to) = parse_date(&rule.effective_to) else {
                    return false;
                };
                effective_from <= period_end && effective_to >= period_start
            })
            .max_by_key(|rule| rule.created_at)
            .ok_or(BillingError::NotFound)?;
        let amount_cents = rule
            .unit_price_cents
            .checked_mul(req.quantity)
            .ok_or(BillingError::InvalidQuantity)?;

        Ok(BillingChargeCalculation {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            contract_id: contract.id,
            period_start: period_start.to_string(),
            period_end: period_end.to_string(),
            charge_item: req.charge_item,
            quantity: req.quantity,
            amount_cents,
            source_refs: req.source_refs,
            status: "calculated".to_string(),
            created_at: now,
        })
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, BillingError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| BillingError::InvalidEffectiveWindow)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        CalculateBillingChargesRequest, CreateBillingAccountRequest, CreateBillingContractRequest,
        CreateBillingRuleRequest,
    };

    use super::{BillingError, BillingStore};
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m9.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn billing_account_contract_and_rule_are_owner_scoped() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 15, 0, 0)
            .single()
            .expect("valid time");
        let ctx_a = ctx(Uuid::new_v4());
        let ctx_b = ctx(Uuid::new_v4());
        let mut store = BillingStore::default();

        let account = store
            .create_account(
                &ctx_a,
                CreateBillingAccountRequest {
                    account_code: "OWNER-A-BILL".to_string(),
                    account_name: "Owner A Billing".to_string(),
                },
                now,
            )
            .expect("account");
        let contract = store
            .create_contract(
                &ctx_a,
                CreateBillingContractRequest {
                    account_id: account.id,
                    contract_no: "CONTRACT-001".to_string(),
                    valid_from: "2026-06-01".to_string(),
                    valid_to: "2027-05-31".to_string(),
                },
                now,
            )
            .expect("contract");
        let rule = store
            .create_rule(
                &ctx_a,
                CreateBillingRuleRequest {
                    contract_id: contract.id,
                    charge_item: "inbound_operation".to_string(),
                    unit: "order".to_string(),
                    unit_price_cents: 100,
                    billing_cycle: "monthly".to_string(),
                    effective_from: "2026-06-01".to_string(),
                    effective_to: "2026-06-30".to_string(),
                },
                now,
            )
            .expect("rule");

        assert_eq!(rule.unit_price_cents, 100);
        assert_eq!(store.list_accounts(&ctx_a).len(), 1);
        assert!(store.list_accounts(&ctx_b).is_empty());
    }

    #[test]
    fn billing_rule_rejects_negative_rate() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 15, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = BillingStore::default();
        let account = store
            .create_account(
                &ctx,
                CreateBillingAccountRequest {
                    account_code: "OWNER-A-BILL".to_string(),
                    account_name: "Owner A Billing".to_string(),
                },
                now,
            )
            .expect("account");
        let contract = store
            .create_contract(
                &ctx,
                CreateBillingContractRequest {
                    account_id: account.id,
                    contract_no: "CONTRACT-001".to_string(),
                    valid_from: "2026-06-01".to_string(),
                    valid_to: "2027-05-31".to_string(),
                },
                now,
            )
            .expect("contract");

        let result = store.create_rule(
            &ctx,
            CreateBillingRuleRequest {
                contract_id: contract.id,
                charge_item: "storage".to_string(),
                unit: "pallet_day".to_string(),
                unit_price_cents: -1,
                billing_cycle: "monthly".to_string(),
                effective_from: "2026-06-01".to_string(),
                effective_to: "2026-06-30".to_string(),
            },
            now,
        );

        assert!(matches!(result, Err(BillingError::InvalidRate)));
    }

    #[test]
    fn billing_effective_windows_reject_invalid_or_overlapping_ranges() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 15, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = BillingStore::default();
        let account = store
            .create_account(
                &ctx,
                CreateBillingAccountRequest {
                    account_code: "OWNER-A-BILL".to_string(),
                    account_name: "Owner A Billing".to_string(),
                },
                now,
            )
            .expect("account");

        let invalid_contract = store.create_contract(
            &ctx,
            CreateBillingContractRequest {
                account_id: account.id,
                contract_no: "CONTRACT-INVALID".to_string(),
                valid_from: "2027-01-01".to_string(),
                valid_to: "2026-01-01".to_string(),
            },
            now,
        );
        assert!(matches!(
            invalid_contract,
            Err(BillingError::InvalidEffectiveWindow)
        ));

        let contract = store
            .create_contract(
                &ctx,
                CreateBillingContractRequest {
                    account_id: account.id,
                    contract_no: "CONTRACT-VALID".to_string(),
                    valid_from: "2026-06-01".to_string(),
                    valid_to: "2027-05-31".to_string(),
                },
                now,
            )
            .expect("contract");
        store
            .create_rule(
                &ctx,
                CreateBillingRuleRequest {
                    contract_id: contract.id,
                    charge_item: "storage".to_string(),
                    unit: "pallet_day".to_string(),
                    unit_price_cents: 100,
                    billing_cycle: "monthly".to_string(),
                    effective_from: "2026-06-01".to_string(),
                    effective_to: "2026-06-30".to_string(),
                },
                now,
            )
            .expect("first rule");

        let overlapping = store.create_rule(
            &ctx,
            CreateBillingRuleRequest {
                contract_id: contract.id,
                charge_item: "storage".to_string(),
                unit: "pallet_day".to_string(),
                unit_price_cents: 110,
                billing_cycle: "monthly".to_string(),
                effective_from: "2026-06-15".to_string(),
                effective_to: "2026-07-15".to_string(),
            },
            now,
        );
        assert!(matches!(
            overlapping,
            Err(BillingError::BillingRuleConflict)
        ));

        let next_window = store
            .create_rule(
                &ctx,
                CreateBillingRuleRequest {
                    contract_id: contract.id,
                    charge_item: "storage".to_string(),
                    unit: "pallet_day".to_string(),
                    unit_price_cents: 120,
                    billing_cycle: "monthly".to_string(),
                    effective_from: "2026-07-01".to_string(),
                    effective_to: "2026-07-31".to_string(),
                },
                now,
            )
            .expect("next window");
        assert_eq!(next_window.unit_price_cents, 120);
    }

    #[test]
    fn calculate_period_charges_uses_owner_scoped_effective_rule() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 15, 0, 0)
            .single()
            .expect("valid time");
        let ctx_a = ctx(Uuid::new_v4());
        let ctx_b = ctx(Uuid::new_v4());
        let mut store = BillingStore::default();

        let account = store
            .create_account(
                &ctx_a,
                CreateBillingAccountRequest {
                    account_code: "OWNER-A-BILL".to_string(),
                    account_name: "Owner A Billing".to_string(),
                },
                now,
            )
            .expect("account");
        let contract = store
            .create_contract(
                &ctx_a,
                CreateBillingContractRequest {
                    account_id: account.id,
                    contract_no: "CONTRACT-001".to_string(),
                    valid_from: "2026-06-01".to_string(),
                    valid_to: "2027-05-31".to_string(),
                },
                now,
            )
            .expect("contract");
        store
            .create_rule(
                &ctx_a,
                CreateBillingRuleRequest {
                    contract_id: contract.id,
                    charge_item: "packing_operation".to_string(),
                    unit: "job".to_string(),
                    unit_price_cents: 125,
                    billing_cycle: "monthly".to_string(),
                    effective_from: "2026-06-01".to_string(),
                    effective_to: "2026-06-30".to_string(),
                },
                now,
            )
            .expect("rule");

        let charge = store
            .calculate_period_charges(
                &ctx_a,
                CalculateBillingChargesRequest {
                    contract_id: contract.id,
                    period_start: "2026-06-01".to_string(),
                    period_end: "2026-06-30".to_string(),
                    charge_item: "packing_operation".to_string(),
                    quantity: 4,
                    source_refs: vec!["packing_job:W5-001".to_string()],
                },
                now,
            )
            .expect("charge");
        assert_eq!(charge.amount_cents, 500);
        assert_eq!(charge.source_refs, vec!["packing_job:W5-001"]);

        let cross_owner = store.calculate_period_charges(
            &ctx_b,
            CalculateBillingChargesRequest {
                contract_id: contract.id,
                period_start: "2026-06-01".to_string(),
                period_end: "2026-06-30".to_string(),
                charge_item: "packing_operation".to_string(),
                quantity: 4,
                source_refs: vec![],
            },
            now,
        );
        assert!(matches!(cross_owner, Err(BillingError::NotFound)));
    }
}
