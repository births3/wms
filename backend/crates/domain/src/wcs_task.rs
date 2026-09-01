//! T03：设备指令任务（wcs_tasks）状态机与校验纯函数。

/// 六态状态机迁移（规格 §5 / §10）：
/// pending → sent → executing → succeeded；异常态 failed / timeout；
/// sent/executing/timeout → sent（重试）；任意非终态 → failed。
pub fn can_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "pending" => matches!(to, "sent" | "failed"),
        "sent" => matches!(to, "executing" | "timeout" | "failed" | "succeeded"),
        "executing" => matches!(to, "succeeded" | "timeout" | "failed" | "sent"),
        "timeout" => matches!(to, "sent" | "failed" | "succeeded"),
        _ => false, // succeeded / failed 为终态，不可再迁移
    }
}

/// 重试仅允许从 sent / executing / timeout 发起（I9）。
pub fn retry_allowed(status: &str) -> bool {
    matches!(status, "sent" | "executing" | "timeout")
}

/// 终态判定。
pub fn is_terminal(status: &str) -> bool {
    matches!(status, "succeeded" | "failed")
}

/// 人工重发（规格 §10.5）：仅 failed / timeout 可重置重试重新入队。
pub fn resend_allowed(status: &str) -> bool {
    matches!(status, "failed" | "timeout")
}

/// 跳过确认（规格 §10.5）：sent / executing / timeout / failed 可补录；
/// pending 尚未派发、succeeded 已落账，均不可跳过。
pub fn confirm_skip_allowed(status: &str) -> bool {
    matches!(status, "sent" | "executing" | "timeout" | "failed")
}

/// DWS 称重校验：pass=true 且重量在预估 ±20% 内（规格 §10.2）。
pub fn dws_result_passes(pass: bool, weight_g: i64, expected_weight_g: i64) -> bool {
    if !pass || expected_weight_g <= 0 {
        return false;
    }
    let diff = (weight_g - expected_weight_g).abs();
    let threshold = expected_weight_g / 5; // ±20%
    diff <= threshold
}

/// RFID EPC 覆盖校验：扫描集合须覆盖目标集合（规格 §10.2）。
pub fn rfid_epcs_cover(target_epcs: &[String], scanned_epcs: &[String]) -> bool {
    target_epcs.iter().all(|epc| scanned_epcs.contains(epc))
}

/// PTL 拍灯数量差异阈值判定（规格 §10.3）：
/// 未超阈值（|Δ|/提示 ≤ ratio 且 |Δ| ≤ abs）→ 允许按拍灯量落账。
pub fn ptl_qty_diff_within_threshold(
    expected: i64,
    pressed: i64,
    ratio: f64,
    max_abs: i64,
) -> bool {
    if expected <= 0 {
        return pressed == 0;
    }
    let diff = (pressed - expected).abs();
    if diff > max_abs {
        return false;
    }
    let diff_ratio = diff as f64 / expected as f64;
    diff_ratio <= ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epc(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn state_machine_allows_spec_transitions() {
        assert!(can_transition("pending", "sent"));
        assert!(can_transition("sent", "executing"));
        assert!(can_transition("executing", "succeeded"));
        assert!(can_transition("sent", "timeout"));
        assert!(can_transition("executing", "timeout"));
        assert!(can_transition("timeout", "sent"));
        assert!(can_transition("pending", "failed"));
        assert!(can_transition("sent", "failed"));
        assert!(can_transition("executing", "failed"));
    }

    #[test]
    fn state_machine_blocks_terminal_and_illegal() {
        assert!(!can_transition("succeeded", "sent"));
        assert!(!can_transition("failed", "sent"));
        assert!(!can_transition("pending", "executing"));
        assert!(!can_transition("succeeded", "failed"));
    }

    #[test]
    fn retry_only_from_active_states() {
        assert!(retry_allowed("sent"));
        assert!(retry_allowed("executing"));
        assert!(retry_allowed("timeout"));
        assert!(!retry_allowed("pending"));
        assert!(!retry_allowed("succeeded"));
        assert!(!retry_allowed("failed"));
    }

    #[test]
    fn resend_only_from_failed_or_timeout() {
        assert!(resend_allowed("failed"));
        assert!(resend_allowed("timeout"));
        assert!(!resend_allowed("pending"));
        assert!(!resend_allowed("sent"));
        assert!(!resend_allowed("succeeded"));
    }

    #[test]
    fn confirm_skip_blocks_already_settled() {
        assert!(confirm_skip_allowed("failed"));
        assert!(confirm_skip_allowed("timeout"));
        assert!(confirm_skip_allowed("executing"));
        assert!(confirm_skip_allowed("sent"));
        assert!(!confirm_skip_allowed("pending"));
        assert!(!confirm_skip_allowed("succeeded"));
    }

    #[test]
    fn dws_pass_within_20_percent() {
        assert!(dws_result_passes(true, 3520, 3500));
        assert!(dws_result_passes(true, 4200, 3500));
        assert!(dws_result_passes(true, 2800, 3500));
        assert!(!dws_result_passes(true, 4300, 3500));
        assert!(!dws_result_passes(false, 3520, 3500));
    }

    #[test]
    fn rfid_cover_requires_all_target_epcs() {
        assert!(rfid_epcs_cover(&epc(&["A", "B"]), &epc(&["A", "B", "C"])));
        assert!(!rfid_epcs_cover(&epc(&["A", "B"]), &epc(&["A", "C"])));
        assert!(rfid_epcs_cover(&epc(&[]), &epc(&[])));
    }

    #[test]
    fn ptl_diff_threshold_rules() {
        assert!(ptl_qty_diff_within_threshold(10, 10, 0.2, 10));
        assert!(ptl_qty_diff_within_threshold(10, 11, 0.2, 10));
        assert!(!ptl_qty_diff_within_threshold(10, 1, 0.2, 10)); // ratio 0.9 > 0.2
        assert!(!ptl_qty_diff_within_threshold(10, 12, 0.2, 1)); // 绝对值超限
        assert!(!ptl_qty_diff_within_threshold(10, 5, 0.2, 10)); // 比例超限
    }
}
