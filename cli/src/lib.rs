pub mod app;
mod vpk;
pub mod mission;

use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

pub(crate) fn get_vpks() -> color_eyre::Result<(
    indexmap::IndexMap<String, PathBuf>,
    HashMap<String, SystemTime>,
    HashMap<String, SystemTime>,
)> {
    let mut current_dir = std::env::current_dir()?;
    if cfg!(debug_assertions) {
        if cfg!(target_os = "windows") {
            current_dir = PathBuf::from(
                r"C:\Program Files (x86)\Steam\steamapps\common\Left 4 Dead 2\left4dead2\addons",
            );
        }
        if cfg!(target_os = "linux") {
            current_dir = PathBuf::from(
                r"/home/cole/.steam/steam/steamapps/common/Left 4 Dead 2/left4dead2/addons",
            )
        }
    }

    let entries = fs::read_dir(current_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && path.extension().unwrap_or_default() == "vpk"
        })
        .collect::<Vec<_>>();

    let mut files = indexmap::IndexMap::new();
    let mut created_times = HashMap::new();
    let mut modified_times = HashMap::new();

    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        if let Ok(meta) = entry.metadata() {
            if let Ok(c) = meta.created() {
                created_times.insert(name.clone(), c);
            }
            if let Ok(m) = meta.modified() {
                modified_times.insert(name.clone(), m);
            }
        }
        files.insert(name, path);
    }

    Ok((files, created_times, modified_times))
}
