use askama::Template;

#[derive(Clone)]
pub struct AdminToastItem {
    pub id: String,
    pub kind: &'static str,
    pub text: String,
    pub ttl_ms: u64,
}

#[derive(Template)]
#[template(path = "admin/toast_stack.html")]
pub struct AdminToastStackTemplate {
    pub toasts: Vec<AdminToastItem>,
}
