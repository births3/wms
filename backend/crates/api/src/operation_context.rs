//! Transport-independent actor and owner scope carried into application code.

use uuid::Uuid;

/// 已完成传输层鉴权后的操作人上下文。
///
/// 该值对象不依赖 Axum、Redis、HTTP 或环境变量；runtime 层的
/// `AuthContext` 只保留为兼容别名并负责从 JWT 构造它。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub actor_name: String,
    pub permissions: Vec<String>,
    pub jti: String,
    /// 外部 API Key 绑定的仓库范围；JWT 会话为 None。
    pub warehouse_scope: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::OperationContext;
    use uuid::Uuid;

    #[test]
    fn preserves_actor_and_owner_scope_without_runtime_dependencies() {
        let user_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let context = OperationContext {
            user_id,
            owner_id,
            actor_name: "alice".to_string(),
            permissions: vec!["inventory:read".to_string()],
            jti: "jti-1".to_string(),
            warehouse_scope: Some(Uuid::new_v4()),
        };

        assert_eq!(context.user_id, user_id);
        assert_eq!(context.owner_id, owner_id);
        assert_eq!(context.actor_name, "alice");
        assert_eq!(context.permissions, ["inventory:read"]);
        assert!(context.warehouse_scope.is_some());
    }
}
