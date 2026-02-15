use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;

pub async fn ensure_export_root(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .await
        .with_context(|| format!("Failed to create export root at {}", path.display()))
}

pub fn build_job_file_name(source_base_name: &str, date_stamp: &str) -> String {
    format!("{source_base_name}-gitnexus-mem_capsule-{date_stamp}.mv2")
}

pub fn job_output_path(export_root: &Path, job_id: &str, file_name: &str) -> PathBuf {
    export_root.join(job_id).join(file_name)
}

pub async fn ensure_job_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create job directory {}", parent.display()))?;
    }
    Ok(())
}

pub async fn delete_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("Failed to delete {}", path.display())),
    }
}
