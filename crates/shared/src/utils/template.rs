use askama::{Error, Template};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize)]
pub struct EmailTemplateData {
    pub title: String,
    pub message: String,
    pub button: String,
    pub link: String,
}

#[derive(Template, Debug)]
#[template(path = "email.html")]
pub struct EmailTemplate<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub button: &'a str,
    pub link: &'a str,
}

impl<'a> From<&'a EmailTemplateData> for EmailTemplate<'a> {
    fn from(data: &'a EmailTemplateData) -> Self {
        EmailTemplate {
            title: data.title.as_str(),
            message: data.message.as_str(),
            button: data.button.as_str(),
            link: data.link.as_str(),
        }
    }
}

pub fn render_email(data: &EmailTemplateData) -> Result<String, Error> {
    info!("📧 Rendering email template: {:?}", data);

    let template = EmailTemplate::from(data);
    match template.render() {
        Ok(result) => {
            info!("✅ Successfully rendered email template.");
            Ok(result)
        }
        Err(e) => {
            error!("❌ Failed to render email template: {}", e);
            Err(e)
        }
    }
}
