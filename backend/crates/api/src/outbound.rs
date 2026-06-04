//! M4 outbound pure domain checks.

use wms_domain::OutboundOrderLine;

pub const OUTBOUND_STATUS_CONFIRMED: &str = "confirmed";
pub const OUTBOUND_STATUS_IN_WAVE: &str = "in_wave";
pub const OUTBOUND_STATUS_PICKED: &str = "picked";
pub const OUTBOUND_STATUS_PICKED_SHORT: &str = "picked_short";
pub const OUTBOUND_STATUS_REVIEWED: &str = "reviewed";
pub const OUTBOUND_STATUS_REVIEWED_SHORT: &str = "reviewed_short";
pub const OUTBOUND_STATUS_SHIPPED: &str = "shipped";

#[derive(Clone, Debug, Default)]
pub struct OutboundOrderStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutboundOrderError {
    EmptyLines,
    InvalidQuantity,
    ShortPickNotReplenished,
}

pub fn short_pick_qty(planned_qty: i64, picked_qty: i64) -> Result<i64, OutboundOrderError> {
    if planned_qty <= 0 || picked_qty < 0 || picked_qty > planned_qty {
        return Err(OutboundOrderError::InvalidQuantity);
    }
    Ok(planned_qty - picked_qty)
}

pub fn all_lines_fulfilled(lines: &[OutboundOrderLine]) -> bool {
    !lines.is_empty() && lines.iter().all(|line| line.picked_qty == line.planned_qty)
}

pub fn all_lines_reviewed_for_ship(lines: &[OutboundOrderLine]) -> Result<(), OutboundOrderError> {
    if lines.is_empty() {
        return Err(OutboundOrderError::EmptyLines);
    }
    if lines
        .iter()
        .any(|line| line.reviewed_qty != line.planned_qty || line.short_pick_qty > 0)
    {
        return Err(OutboundOrderError::ShortPickNotReplenished);
    }
    Ok(())
}

pub fn status_after_pick(lines: &[OutboundOrderLine]) -> &'static str {
    if all_lines_fulfilled(lines) {
        OUTBOUND_STATUS_PICKED
    } else {
        OUTBOUND_STATUS_PICKED_SHORT
    }
}

pub fn status_after_review(lines: &[OutboundOrderLine]) -> &'static str {
    if lines
        .iter()
        .all(|line| line.reviewed_qty == line.planned_qty && line.short_pick_qty == 0)
    {
        OUTBOUND_STATUS_REVIEWED
    } else {
        OUTBOUND_STATUS_REVIEWED_SHORT
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wms_domain::OutboundOrderLine;

    use super::{
        all_lines_reviewed_for_ship, short_pick_qty, status_after_pick, OutboundOrderError,
        OUTBOUND_STATUS_PICKED, OUTBOUND_STATUS_PICKED_SHORT,
    };

    fn line(planned_qty: i64, picked_qty: i64, reviewed_qty: i64) -> OutboundOrderLine {
        OutboundOrderLine {
            line_no: 1,
            product_code: "P-001".to_string(),
            batch_no: format!("B-{}", Uuid::new_v4()),
            planned_qty,
            picked_qty,
            reviewed_qty,
            shipped_qty: 0,
            short_pick_qty: planned_qty - picked_qty,
        }
    }

    #[test]
    fn short_pick_can_continue_but_cannot_ship_until_replenished() {
        let short = vec![line(10, 8, 8)];

        assert_eq!(short_pick_qty(10, 8), Ok(2));
        assert_eq!(status_after_pick(&short), OUTBOUND_STATUS_PICKED_SHORT);
        assert!(matches!(
            all_lines_reviewed_for_ship(&short),
            Err(OutboundOrderError::ShortPickNotReplenished)
        ));

        let replenished = vec![line(10, 10, 10)];
        assert_eq!(status_after_pick(&replenished), OUTBOUND_STATUS_PICKED);
        assert_eq!(all_lines_reviewed_for_ship(&replenished), Ok(()));
    }
}
