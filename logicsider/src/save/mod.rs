use super::*;

#[derive(Serialize, Deserialize)]
enum Versioned<'a> {
    V0(Cow<'a, GameState>),
}

impl GameState {
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        // TODO figure out the web
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        ron::ser::to_writer_pretty(writer, &Versioned::V0(Cow::Borrowed(self)), default())?;
        Ok(())
    }

    pub async fn load_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut versioned: Versioned<'static> = match file::load_detect(path).await {
            Ok(versioned) => versioned,
            Err(_) => Versioned::V0(file::load_detect(path).await?),
        };
        if let Versioned::V0(state) = versioned {
            Ok(state.into_owned())
        } else {
            unreachable!()
        }
    }
}
