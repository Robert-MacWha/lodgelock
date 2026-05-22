use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PluginSource {
    Embedded(Vec<u8>),
    Url(String),
}

impl PluginSource {
    pub async fn as_bytes(&self) -> Result<Vec<u8>, reqwest::Error> {
        match self {
            PluginSource::Embedded(bytes) => Ok(bytes.clone()),
            PluginSource::Url(url) => {
                let response = reqwest::get(url).await?;
                let response = response.error_for_status()?;
                let bytes = response.bytes().await?;
                Ok(bytes.to_vec())
            }
        }
    }
}
