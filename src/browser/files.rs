use std::path::PathBuf;

#[allow(dead_code)]
pub enum FileOp {
    Rename { old: PathBuf, new: String },
    Delete { path: PathBuf },
    Copy { path: PathBuf },
    OpenExternal { path: PathBuf },
}

#[allow(dead_code)]
pub fn execute(op: FileOp) -> Result<(), String> {
    match op {
        FileOp::Rename { old, new } => {
            let new_path = old.with_file_name(&new);
            std::fs::rename(&old, &new_path).map_err(|e| format!("Rename failed: {e}"))
        }
        FileOp::Delete { path } => {
            std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))
        }
        FileOp::Copy { path } => {
            if let Some(name) = path.file_name() {
                let mut dest = path.with_file_name(format!("Copy_of_{}", name.to_string_lossy()));
                let mut n = 1;
                while dest.exists() {
                    dest = path
                        .with_file_name(format!("Copy_of_{}({})", name.to_string_lossy(), n));
                    n += 1;
                }
                std::fs::copy(&path, &dest).map_err(|e| format!("Copy failed: {e}"))?;
            }
            Ok(())
        }
        FileOp::OpenExternal { path } => {
            open::that(&path).map_err(|e| format!("Open failed: {e}"))
        }
    }
}
