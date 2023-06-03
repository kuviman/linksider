#[test]
fn main() {
    let file = std::fs::File::open(
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("..")
            .join("assets")
            .join("world.ldtk"),
    )
    .unwrap();
    let reader = std::io::BufReader::new(file);
    let ldtk: ldtk_json::Ldtk = serde_json::from_reader(reader).unwrap();
    eprintln!("{ldtk:#?}");
}
