use crate::{invalid, OmrService, Result, UiPreferences, OSSIA_OMR_URL};

#[derive(Clone)]
pub(crate) struct OmrConfig {
    pub url: String,
    pub token: String,
}

impl OmrConfig {
    pub fn from_preferences(preferences: &UiPreferences) -> Result<Self> {
        let (url, token) = match preferences.omr_service {
            OmrService::Ossia => (OSSIA_OMR_URL.to_string(), preferences.omr_token.as_deref()),
            OmrService::Custom => {
                let url = preferences
                    .omr_url
                    .as_deref()
                    .ok_or_else(|| invalid("Custom remote OMR URL is not configured"))?;
                (
                    crate::preferences::validate_omr_url(url)?,
                    preferences.omr_custom_token.as_deref(),
                )
            }
        };
        let token = token.ok_or_else(|| invalid("Remote OMR token is not configured"))?;
        let token = crate::preferences::normalize_omr_token(token)?;
        Ok(Self { url, token })
    }
}
