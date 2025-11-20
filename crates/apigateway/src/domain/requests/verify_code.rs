use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct VerifyCodeQuery {
    pub verify_code: String,
}
