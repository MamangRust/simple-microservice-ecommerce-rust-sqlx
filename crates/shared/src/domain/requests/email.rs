use crate::utils::EmailTemplateData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailRequest {
    pub to: String,
    pub subject: String,
    pub data: EmailTemplateData,
}
