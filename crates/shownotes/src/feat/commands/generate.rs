use error_stack::{Report, ResultExt};

use crate::{
    feat::{generate_show_notes, playlist::PlaylistData},
    services::Services,
};

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct GenerateError;

pub async fn generate_notes(
    services: &Services,
    playlist_data: &PlaylistData,
    format: &str,
) -> Result<String, Report<GenerateError>> {
    generate_show_notes(playlist_data, &services.sources, format)
        .await
        .change_context(GenerateError)
}
