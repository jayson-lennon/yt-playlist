use tempfile::NamedTempFile;

pub fn create_temp_file() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}
