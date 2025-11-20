use serde::{Deserialize, Serialize};
use shared::utils::EmailTemplateData;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailRequest {
    pub to: String,
    pub subject: String,
    pub data: EmailTemplateData,
}
