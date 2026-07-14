use super::*;

impl ReceivingOrderStore {
    pub fn get_print_data(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<ReceivingOrderPrintData, ReceivingOrderError> {
        let order = self.get(ctx, id)?;
        let mut receipts: Vec<_> = self
            .receipts
            .values()
            .filter(|receipt| receipt.receiving_order_id == id && receipt.owner_id == ctx.owner_id)
            .cloned()
            .collect();
        receipts.sort_by_key(|receipt| (receipt.occurred_at, receipt.id));
        let mut inspections: Vec<_> = self
            .inspections
            .values()
            .filter(|inspection| {
                inspection.receiving_order_id == id && inspection.owner_id == ctx.owner_id
            })
            .cloned()
            .collect();
        inspections.sort_by_key(|inspection| (inspection.occurred_at, inspection.id));
        let mut signatures: Vec<_> = self
            .signatures
            .values()
            .filter(|signature| {
                signature.receiving_order_id == id && signature.owner_id == ctx.owner_id
            })
            .cloned()
            .collect();
        signatures.sort_by_key(|signature| (signature.signed_at, signature.id));
        Ok(ReceivingOrderPrintData {
            order,
            receipts,
            inspections,
            signatures,
        })
    }
}
