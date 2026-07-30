export interface PortalUser {
  id: string;
  customer_id: string;
  username: string;
  display_name: string;
  role: "customer_admin" | "customer_user";
  status: "active" | "disabled" | "locked";
  can_view_report_history: boolean;
  address_ids: string[];
}

export interface LoginResponse {
  access_token: string;
  expires_at: string;
  user: PortalUser;
}

export interface Address {
  id: string;
  address_code: string;
  address_name: string;
}

export interface OrderSummary {
  id: string;
  order_no: string;
  status: "shipped" | "signed";
  customer_code: string;
  customer_name: string;
  delivery_address_id: string;
  address_code: string;
  address_name: string;
  product_codes: string[];
  product_names: string[];
  batch_nos: string[];
  quantities: number[];
  shipped_at: string;
  signed_at: string | null;
  line_count: number;
  available_report_count: number;
  pending_report_count: number;
}

export interface ReportSummary {
  id: string;
  report_id: string;
  version_number: number;
  report_no: string;
  status: "confirmed" | "superseded";
  is_current: boolean;
  modification_reason: string | null;
  customer_copy_status: "queued" | "processing" | "available" | "failed";
  customer_copy_file_name: string | null;
  customer_copy_size: number | null;
  digitally_signed_original: boolean;
  confirmed_at: string;
}

export interface OrderLine {
  id: string;
  product_id: string;
  product_code: string;
  product_name: string;
  batch_no: string;
  quantity: number;
  reports: ReportSummary[];
}

export interface OrderDetail {
  id: string;
  order_no: string;
  status: "shipped" | "signed";
  delivery_address_id: string;
  address_snapshot: Record<string, unknown>;
  shipped_at: string;
  signed_at: string | null;
  lines: OrderLine[];
}

export interface ExportJob {
  id: string;
  include_history: boolean;
  status: "queued" | "processing" | "completed" | "failed";
  requested_order_count: number;
  report_file_count: number;
  missing_count: number;
  total_size: number;
  result_file_name: string | null;
  last_error: string | null;
  expires_at: string | null;
  created_at: string;
  finished_at: string | null;
}

export interface DownloadUrl {
  url: string;
  file_name: string;
  expires_at: string;
}
