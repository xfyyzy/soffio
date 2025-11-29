use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiKeyFilters {
    pub status: Option<String>,
    pub search: Option<String>,
    pub scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyForm {
    pub name: String,
    pub description: Option<String>,
    pub scope_state: Option<String>,
    pub expires_in: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyIdForm {
    pub id: String,
    pub status_filter: Option<String>,
    pub filter_search: Option<String>,
    pub filter_scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiKeyPanelForm {
    pub status: Option<String>,
    pub search: Option<String>,
    pub scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScopeToggleForm {
    pub scope_id: String,
    pub scope_state: Option<String>,
}
