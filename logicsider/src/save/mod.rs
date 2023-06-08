use super::*;

impl GameState {
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        // TODO figure out the web
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        ron::ser::to_writer_pretty(writer, self, default())?;
        Ok(())
    }

    pub async fn load_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        file::load_detect(path).await
    }
}
