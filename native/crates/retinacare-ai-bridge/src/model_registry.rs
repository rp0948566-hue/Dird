//! Bounded local model catalog discovery. Hashing remains part of load, not scan.
use crate::model_manifest::{ArtifactFormat, inspect_manifest};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

const MAX_SCAN_DEPTH: usize = 4;
const MAX_MANIFESTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    AwaitingIntegrityVerification,
    ConversionRequired,
    InvalidManifest(String),
    DuplicateId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalogEntry {
    pub manifest_path: PathBuf,
    pub model_id: Option<String>,
    pub version: Option<String>,
    pub format: Option<ArtifactFormat>,
    pub file_size_bytes: Option<u64>,
    pub status: ModelStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRegistry {
    pub entries: Vec<ModelCatalogEntry>,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("model directory cannot be read: {0}")]
    Read(#[from] std::io::Error),
    #[error("model manifest scan exceeds {MAX_MANIFESTS} files")]
    TooManyManifests,
    #[error("model registry root cannot be a symlink")]
    SymlinkRoot,
}

impl ModelRegistry {
    /// Discover `manifest.json` in model directories and `*.model.json` files.
    /// Symlinked folders are ignored; scan depth and count are bounded.
    pub fn discover(root: &Path) -> Result<Self, RegistryError> {
        if !root.exists() {
            return Ok(Self {
                entries: Vec::new(),
            });
        }
        if fs::symlink_metadata(root)?.file_type().is_symlink() {
            return Err(RegistryError::SymlinkRoot);
        }
        let mut manifests = Vec::new();
        visit(root, 0, &mut manifests)?;
        manifests.sort();

        let mut seen_ids = std::collections::HashSet::new();
        let entries = manifests
            .into_iter()
            .map(|manifest_path| match inspect_manifest(&manifest_path) {
                Ok(model) => {
                    let status = if !seen_ids.insert(model.model_id.clone()) {
                        ModelStatus::DuplicateId
                    } else if model.format == ArtifactFormat::PyTorch {
                        ModelStatus::ConversionRequired
                    } else {
                        ModelStatus::AwaitingIntegrityVerification
                    };
                    ModelCatalogEntry {
                        manifest_path,
                        model_id: Some(model.model_id),
                        version: Some(model.version),
                        format: Some(model.format),
                        file_size_bytes: Some(model.file_size_bytes),
                        status,
                    }
                }
                Err(error) => ModelCatalogEntry {
                    manifest_path,
                    model_id: None,
                    version: None,
                    format: None,
                    file_size_bytes: None,
                    status: ModelStatus::InvalidManifest(error.to_string()),
                },
            })
            .collect();
        Ok(Self { entries })
    }
}

fn visit(
    directory: &Path,
    depth: usize,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), RegistryError> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            visit(&path, depth + 1, manifests)?;
        } else if file_type.is_file()
            && (entry.file_name() == "manifest.json"
                || entry.file_name().to_string_lossy().ends_with(".model.json"))
        {
            if manifests.len() == MAX_MANIFESTS {
                return Err(RegistryError::TooManyManifests);
            }
            manifests.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn model_files(directory: &Path, name: &str, extension: &str) {
        fs::create_dir_all(directory).unwrap();
        let bytes = b"catalog-only-test-artifact";
        let file = format!("{name}.{extension}");
        fs::write(directory.join(&file), bytes).unwrap();
        fs::write(
            directory.join("manifest.json"),
            json!({
                "schema_version": 1,
                "model_id": name,
                "version": "1.0.0",
                "task": "synthetic_test",
                "architecture": "test",
                "checkpoint": file,
                "size_bytes": bytes.len(),
                "sha256": hex::encode(Sha256::digest(bytes)),
                "license": "CC0-1.0",
                "source_url": "https://example.org/test-model",
                "input": {
                    "name": "input",
                    "data_type": "float32",
                    "shape": [1, 1, 1, 1],
                    "color_space": "RGB",
                    "normalization": "none"
                },
                "outputs": {
                    "schema": "scalar",
                    "name": "output",
                    "data_type": "float32",
                    "shape": [1, 1]
                },
                "validation": {"report": "synthetic"}
            })
            .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn discovers_onnx_and_marks_pytorch_for_conversion() {
        let root = std::env::temp_dir().join(format!("retinacare-registry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        model_files(&root.join("onnx"), "fixture-onnx", "onnx");
        model_files(&root.join("pytorch"), "fixture-pytorch", "pth");
        let registry = ModelRegistry::discover(&root).unwrap();
        assert_eq!(registry.entries.len(), 2);
        assert!(registry.entries.iter().any(|entry| {
            entry.status == ModelStatus::AwaitingIntegrityVerification
                && entry.format == Some(ArtifactFormat::Onnx)
        }));
        assert!(registry.entries.iter().any(|entry| {
            entry.status == ModelStatus::ConversionRequired
                && entry.format == Some(ArtifactFormat::PyTorch)
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn absent_model_directory_is_an_empty_catalog() {
        let root = std::env::temp_dir().join(format!(
            "retinacare-registry-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        assert!(ModelRegistry::discover(&root).unwrap().entries.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_registry_root() {
        use std::os::unix::fs::symlink;
        let root =
            std::env::temp_dir().join(format!("retinacare-registry-link-{}", std::process::id()));
        let target =
            std::env::temp_dir().join(format!("retinacare-registry-target-{}", std::process::id()));
        let _ = fs::remove_file(&root);
        let _ = fs::remove_dir_all(&target);
        fs::create_dir_all(&target).unwrap();
        symlink(&target, &root).unwrap();
        assert!(matches!(
            ModelRegistry::discover(&root),
            Err(RegistryError::SymlinkRoot)
        ));
        fs::remove_file(root).unwrap();
        fs::remove_dir_all(target).unwrap();
    }
}
