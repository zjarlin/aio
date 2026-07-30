use rudi::Singleton;

/// 管理模式下由 Workbench 消费的程序编辑能力。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAppState {
    pub can_add_scene: bool,
    pub can_add_menu: bool,
    pub can_edit_page: bool,
}

#[Singleton(name = module_path!())]
pub fn admin_app_state() -> AdminAppState {
    AdminAppState {
        can_add_scene: true,
        can_add_menu: true,
        can_edit_page: true,
    }
}

/// 从 Rudi 上下文解析管理模式状态。
pub fn resolve_admin_app_state(context: &mut rudi::Context) -> Option<AdminAppState> {
    context.resolve_option_with_name::<AdminAppState>(module_path!())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rudi_registers_admin_app_state() {
        crate::enable();
        let mut context = rudi::Context::auto_register();
        let state = resolve_admin_app_state(&mut context);

        assert_eq!(
            state,
            Some(AdminAppState {
                can_add_scene: true,
                can_add_menu: true,
                can_edit_page: true,
            })
        );
    }
}
