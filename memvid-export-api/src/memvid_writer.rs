use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use memvid_core::{
    Memvid, PutOptions, MEMVID_EMBEDDING_DIMENSION_KEY, MEMVID_EMBEDDING_MODEL_KEY,
    MEMVID_EMBEDDING_PROVIDER_KEY,
};
use tracing::warn;

use crate::embedding::EmbeddingRuntimeConfig;
use crate::models::FrameDocument;

pub fn write_mv2<F>(
    path: &Path,
    docs: &[FrameDocument],
    semantic_enabled: bool,
    embedding_config: Option<EmbeddingRuntimeConfig>,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(usize, usize),
{
    if let Err(err) = write_with_memvid_core(
        path,
        docs,
        semantic_enabled,
        embedding_config.clone(),
        &mut on_progress,
    ) {
        if semantic_enabled {
            return Err(err);
        }
        warn!("memvid-core write failed, falling back to memvid CLI: {err:#}");
        write_with_memvid_cli(path, docs, &mut on_progress)?;
    }
    Ok(())
}

pub fn write_mv2_core_only<F>(
    path: &Path,
    docs: &[FrameDocument],
    semantic_enabled: bool,
    embedding_config: Option<EmbeddingRuntimeConfig>,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(usize, usize),
{
    write_with_memvid_core(
        path,
        docs,
        semantic_enabled,
        embedding_config,
        &mut on_progress,
    )
}

fn write_with_memvid_core<F>(
    path: &Path,
    docs: &[FrameDocument],
    semantic_enabled: bool,
    embedding_config: Option<EmbeddingRuntimeConfig>,
    on_progress: &mut F,
) -> Result<()>
where
    F: FnMut(usize, usize),
{
    let mut mem =
        Memvid::create(path).with_context(|| format!("Failed to create {}", path.display()))?;

    let embedding_config = if semantic_enabled {
        Some(
            embedding_config
                .context("Semantic export requires valid embedding runtime configuration")?,
        )
    } else {
        None
    };

    let total = docs.len().max(1);
    for (idx, doc) in docs.iter().enumerate() {
        let mut builder = PutOptions::builder()
            .title(doc.title.clone())
            .uri(doc.uri.clone())
            .search_text(doc.text.clone())
            .tag("track".to_string(), doc.track.clone())
            .tag("label".to_string(), doc.label.clone())
            .tag("semantic".to_string(), semantic_enabled.to_string());

        for tag in &doc.tags {
            if let Some((k, v)) = tag.split_once('=') {
                builder = builder.tag(k.to_string(), v.to_string());
            }
        }

        let options = builder.build();
        if let Some(runtime) = embedding_config.as_ref() {
            let embedding = runtime
                .embed_text(&doc.text)
                .with_context(|| format!("Failed generating embedding for frame {}", doc.uri))?;
            let embedding_dims = embedding.len();
            let options_with_identity = PutOptions::builder()
                .title(doc.title.clone())
                .uri(doc.uri.clone())
                .search_text(doc.text.clone())
                .tag("track".to_string(), doc.track.clone())
                .tag("label".to_string(), doc.label.clone())
                .tag("semantic".to_string(), "true".to_string())
                .tag(
                    MEMVID_EMBEDDING_PROVIDER_KEY.to_string(),
                    runtime.provider.as_str().to_string(),
                )
                .tag(
                    MEMVID_EMBEDDING_MODEL_KEY.to_string(),
                    runtime.model.clone(),
                )
                .tag(
                    MEMVID_EMBEDDING_DIMENSION_KEY.to_string(),
                    embedding_dims.to_string(),
                );
            let mut options_with_identity = options_with_identity;
            for tag in &doc.tags {
                if let Some((k, v)) = tag.split_once('=') {
                    options_with_identity = options_with_identity.tag(k.to_string(), v.to_string());
                }
            }
            let options_with_identity = options_with_identity.build();

            mem.put_with_embedding_and_options(
                doc.text.as_bytes(),
                embedding,
                options_with_identity,
            )
            .with_context(|| format!("Failed writing embedded frame {}", doc.uri))?;
        } else {
            mem.put_bytes_with_options(doc.text.as_bytes(), options)
                .with_context(|| format!("Failed writing frame {}", doc.uri))?;
        }
        on_progress(idx + 1, total);
    }

    mem.commit()
        .with_context(|| format!("Failed to commit {}", path.display()))?;
    Ok(())
}

fn write_with_memvid_cli<F>(path: &Path, docs: &[FrameDocument], on_progress: &mut F) -> Result<()>
where
    F: FnMut(usize, usize),
{
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to clear existing {}", path.display()))?;
    }

    let create_status = Command::new("memvid")
        .arg("create")
        .arg(path)
        .status()
        .context("Failed to execute `memvid create`")?;

    if !create_status.success() {
        anyhow::bail!("`memvid create` failed with status {create_status}");
    }

    let total = docs.len().max(1);
    for (idx, doc) in docs.iter().enumerate() {
        let mut command = Command::new("memvid");
        command
            .arg("put")
            .arg(path)
            .arg("--input")
            .arg("-")
            .arg("--title")
            .arg(&doc.title)
            .arg("--track")
            .arg(&doc.track)
            .arg("--tag")
            .arg(format!("label={}", doc.label));

        for tag in &doc.tags {
            command.arg("--tag").arg(tag);
        }

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to execute `memvid put`")?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(doc.text.as_bytes())
                .context("Failed writing frame content to memvid CLI stdin")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed waiting for `memvid put`")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`memvid put` failed for {}: {}", doc.uri, stderr);
        }
        on_progress(idx + 1, total);
    }

    Ok(())
}
