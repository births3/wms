//! 补货仓储所有 FOR UPDATE 必须先 SET LOCAL lock_timeout。

#[test]
fn for_update_sites_call_set_lock_timeout() {
    for (path, src) in [
        (
            "replenishment_repository.rs",
            include_str!("../src/replenishment_repository.rs"),
        ),
        (
            "replenishment_repository_task.rs",
            include_str!("../src/replenishment_repository_task.rs"),
        ),
        (
            "replenishment_repository_strategy.rs",
            include_str!("../src/replenishment_repository_strategy.rs"),
        ),
    ] {
        let updates = src.matches("FOR UPDATE").count();
        let timeouts = src.matches("set_lock_timeout").count();
        assert!(
            updates == 0 || timeouts >= updates,
            "{path}: {updates} FOR UPDATE but {timeouts} set_lock_timeout"
        );
    }
}
