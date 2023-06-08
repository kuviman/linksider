use super::*;

pub mod v0;
pub mod v1;

pub use v1 as current;

#[derive(Serialize, Deserialize)]
enum Versioned {
    V0(v0::GameState),
    V1(v1::Level),
}

impl Level {
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        // TODO figure out the web
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        ron::ser::to_writer_pretty(writer, &Versioned::V1(self.clone()), default())?;
        Ok(())
    }

    pub async fn load_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut versioned: Versioned = match file::load_detect(path).await {
            Ok(versioned) => versioned,
            Err(_) => Versioned::V0(file::load_detect(path).await?),
        };
        if let Versioned::V0(state) = versioned {
            versioned = Versioned::V1(v0::migrate(state));
        }
        if let Versioned::V1(state) = versioned {
            Ok(state)
        } else {
            unreachable!()
        }
    }
}
