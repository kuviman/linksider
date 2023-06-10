use super::*;

pub async fn load_group_names() -> Vec<String> {
    file::load_detect(groups_list_file()).await.unwrap()
}

pub fn save_group_names(group_names: &[&str]) {
    let file = std::fs::File::create(groups_list_file()).unwrap();
    let writer = std::io::BufWriter::new(file);
    ron::ser::to_writer_pretty(writer, group_names, default()).unwrap();
}

pub async fn load_level_names(group_name: &str) -> Vec<String> {
    let list_path = levels_list_file(group_name);
    file::load_detect(list_path).await.unwrap()
}

pub fn save_level_names(group_name: &str, level_names: &[&str]) {
    let file = std::fs::File::create(levels_list_file(group_name)).unwrap();
    let writer = std::io::BufWriter::new(file);
    ron::ser::to_writer_pretty(writer, level_names, default()).unwrap();
}

pub fn group_dir(group_name: &str) -> std::path::PathBuf {
    run_dir().join("levels").join(group_name)
}

pub fn groups_list_file() -> std::path::PathBuf {
    run_dir().join("levels").join("groups.ron")
}

pub fn levels_list_file(group_name: &str) -> std::path::PathBuf {
    group_dir(group_name).join("list.ron")
}

pub fn level_path(group_name: &str, level_name: &str) -> std::path::PathBuf {
    group_dir(group_name).join(format!("{level_name}.ron"))
}
