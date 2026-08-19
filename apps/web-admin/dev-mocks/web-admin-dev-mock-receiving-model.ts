export interface DevReceivingPrintData {
  receipts: Array<{
    id: string;
    receiving_order_id: string;
    owner_id: string;
    actual_qty: number;
    shortage_qty: number;
    rejected_qty: number;
    arrival_temperature_celsius: number | null;
    exception_note: string | null;
    details: {
      delivery_qty: number;
      temperature_control_method: string | null;
      vehicle_no: string | null;
      origin: string | null;
      departure_at: string | null;
      arrival_at: string | null;
      storage_at: string | null;
      transport_mode: string | null;
      carrier: string | null;
      contact_name: string | null;
      contact_phone: string | null;
      contact_id_no: string | null;
      seal_checked: string | null;
      filing_checked: string | null;
      second_receiver_id: string | null;
      sales_return_batches: Array<{
        batch_no: string;
        quantity: number;
        rejected_qty: number;
        reject_reason: string | null;
      }>;
    } | null;
    occurred_at: string;
  }>;
  inspections: Array<{
    id: string;
    receiving_order_id: string;
    owner_id: string;
    batch_no: string;
    accepted_qty: number;
    rejected_qty: number;
    quality_status: string;
    occurred_at: string;
  }>;
  signatures: Array<{
    id: string;
    receiving_order_id: string;
    owner_id: string;
    first_signer_id: string;
    second_signer_id: string | null;
    signed_at: string;
  }>;
}
