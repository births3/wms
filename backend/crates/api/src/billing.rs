//! Wave 3 M9 billing account and contract model service.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingContract, BillingRule, CreateBillingAccountRequest,
    CreateBillingContractRequest, CreateBillingRuleRequest,
};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BillingError {
    NotFound,
    DuplicateAccountCode(String),
    DuplicateContractNo(String),
    InvalidRate,
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
        let contract = BillingContract {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            account_id: account.id,
            contract_no: req.contract_no,
            valid_from: req.valid_from,
            valid_to: req.valid_to,
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
        let rule = BillingRule {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            contract_id: contract.id,
            charge_item: req.charge_item,
            unit: req.unit,
            unit_price_cents: req.unit_price_cents,
            billing_cycle: req.billing_cycle,
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
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
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
            },
            now,
        );

        assert!(matches!(result, Err(BillingError::InvalidRate)));
    }
}
