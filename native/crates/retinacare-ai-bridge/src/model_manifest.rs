//! Manifest validation and artifact integrity gate for local model files.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    ops::Deref,
    path::{Path, PathBuf},
};
use thiserror::Error;

const SCHEMA_VERSION: u32 = 1;
const SHA256_HEX_LEN: usize = 64;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_MODEL_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_INPUT_ELEMENTS: usize = 256_000_000;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest cannot be read: {0}")]
    Unreadable(#[from] io::Error),
    #[error("manifest is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("manifest exceeds the {MAX_MANIFEST_BYTES}-byte limit")]
    ManifestTooLarge,
    #[error("manifest schema_version {0} is unsupported")]
    UnsupportedSchema(u32),
    #[error("manifest field `{0}` is missing, empty or an unresolved placeholder")]
    InvalidField(&'static str),
    #[error("checkpoint path must be one safe filename")]
    UnsafeCheckpointPath,
    #[error("checkpoint extension is unsupported; expected .onnx, .pt or .pth")]
    UnsupportedArtifactFormat,
    #[error("checkpoint is not a regular, non-symlink file: {0}")]
    CheckpointNotRegular(PathBuf),
    #[error("checkpoint is missing or unreadable: {0}")]
    CheckpointMissing(PathBuf),
    #[error("checkpoint exceeds the {MAX_MODEL_BYTES}-byte limit")]
    CheckpointTooLarge,
    #[error("checkpoint size mismatch: expected {expected} bytes, found {actual}")]
    CheckpointSizeMismatch { expected: u64, actual: u64 },
    #[error("checkpoint sha256 mismatch: expected {expected}, found {actual}")]
    CheckpointHashMismatch { expected: String, actual: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactFormat {
    Onnx,
    PyTorch,
}

impl ArtifactFormat {
    fn from_path(path: &Path) -> Result<Self, ManifestError> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension) if extension.eq_ignore_ascii_case("onnx") => Ok(Self::Onnx),
            Some(extension)
                if extension.eq_ignore_ascii_case("pt")
                    || extension.eq_ignore_ascii_case("pth") =>
            {
                Ok(Self::PyTorch)
            }
            _ => Err(ManifestError::UnsupportedArtifactFormat),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u32,
    model_id: String,
    version: String,
    task: String,
    architecture: String,
    checkpoint: String,
    sha256: String,
    size_bytes: u64,
    license: String,
    source_url: String,
    input: InputSpec,
    outputs: OutputSpec,
    validation: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct InputSpec {
    name: String,
    data_type: String,
    shape: Vec<usize>,
    color_space: String,
    normalization: String,
}

#[derive(Debug, Deserialize)]
struct OutputSpec {
    schema: String,
    name: String,
    data_type: String,
    shape: Vec<i64>,
}

/// Validated metadata. `inspect_manifest` validates shape and local file
/// metadata but does not hash the artifact; `verify_manifest` does both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDescriptor {
    pub model_id: String,
    pub version: String,
    pub task: String,
    pub architecture: String,
    pub format: ArtifactFormat,
    pub manifest_path: PathBuf,
    pub checkpoint: PathBuf,
    pub file_size_bytes: u64,
    pub input_name: String,
    pub input_data_type: String,
    pub input_shape: Vec<usize>,
    pub output_schema: String,
    pub output_name: String,
    pub output_data_type: String,
    pub output_shape: Vec<i64>,
    pub expected_sha256: String,
    pub license: String,
    pub source_url: String,
    pub validation: Option<serde_json::Value>,
}

/// Artifact whose SHA-256 matched its validated manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedModel {
    pub descriptor: ModelDescriptor,
    pub sha256: String,
}

impl Deref for VerifiedModel {
    type Target = ModelDescriptor;

    fn deref(&self) -> &Self::Target {
        &self.descriptor
    }
}

/// Validate manifest metadata and resolve its local checkpoint without hashing
/// it. This is suitable for catalog listing; loading must call
/// `verify_manifest` before passing any ONNX file to ONNX Runtime.
pub fn inspect_manifest(manifest_path: &Path) -> Result<ModelDescriptor, ManifestError> {
    let manifest_metadata = fs::symlink_metadata(manifest_path)?;
    if !manifest_metadata.file_type().is_file() {
        return Err(ManifestError::CheckpointNotRegular(
            manifest_path.to_path_buf(),
        ));
    }
    if manifest_metadata.len() > MAX_MANIFEST_BYTES {
        return Err(ManifestError::ManifestTooLarge);
    }
    let mut bytes = Vec::with_capacity(manifest_metadata.len() as usize);
    fs::File::open(manifest_path)?
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ManifestError::ManifestTooLarge);
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(ManifestError::UnsupportedSchema(manifest.schema_version));
    }
    for (field, value) in [
        ("model_id", &manifest.model_id),
        ("version", &manifest.version),
        ("task", &manifest.task),
        ("architecture", &manifest.architecture),
        ("checkpoint", &manifest.checkpoint),
        ("license", &manifest.license),
        ("source_url", &manifest.source_url),
        ("input.name", &manifest.input.name),
        ("input.data_type", &manifest.input.data_type),
        ("input.color_space", &manifest.input.color_space),
        ("input.normalization", &manifest.input.normalization),
        ("outputs.schema", &manifest.outputs.schema),
        ("outputs.name", &manifest.outputs.name),
        ("outputs.data_type", &manifest.outputs.data_type),
    ] {
        if is_placeholder(value) {
            return Err(ManifestError::InvalidField(field));
        }
    }
    if manifest
        .validation
        .as_ref()
        .is_none_or(|evidence| evidence.is_null())
    {
        return Err(ManifestError::InvalidField("validation"));
    }
    let expected = manifest.sha256.trim().to_ascii_lowercase();
    if expected.len() != SHA256_HEX_LEN || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ManifestError::InvalidField("sha256"));
    }
    let shape = &manifest.input.shape;
    let input_elements = shape
        .iter()
        .try_fold(1usize, |count, dimension| count.checked_mul(*dimension));
    if shape.len() != 4
        || shape[0] != 1
        || shape.contains(&0)
        || input_elements.is_none_or(|elements| elements > MAX_INPUT_ELEMENTS)
        || manifest.input.data_type != "float32"
    {
        return Err(ManifestError::InvalidField("input.shape"));
    }
    if manifest.outputs.data_type != "float32"
        || manifest.outputs.shape.is_empty()
        || manifest
            .outputs
            .shape
            .iter()
            .any(|dimension| *dimension <= 0)
    {
        return Err(ManifestError::InvalidField("outputs"));
    }
    let checkpoint_name = Path::new(&manifest.checkpoint);
    if checkpoint_name.components().count() != 1
        || checkpoint_name.file_name().and_then(|name| name.to_str())
            != Some(manifest.checkpoint.as_str())
        || !manifest
            .checkpoint
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(ManifestError::UnsafeCheckpointPath);
    }
    let format = ArtifactFormat::from_path(checkpoint_name)?;
    let manifest_path = fs::canonicalize(manifest_path)?;
    let manifest_dir = manifest_path
        .parent()
        .ok_or(ManifestError::UnsafeCheckpointPath)?;
    let checkpoint = manifest_dir.join(checkpoint_name);
    let metadata = fs::symlink_metadata(&checkpoint)
        .map_err(|_| ManifestError::CheckpointMissing(checkpoint.clone()))?;
    if !metadata.file_type().is_file() {
        return Err(ManifestError::CheckpointNotRegular(checkpoint));
    }
    if metadata.len() > MAX_MODEL_BYTES {
        return Err(ManifestError::CheckpointTooLarge);
    }
    if metadata.len() != manifest.size_bytes {
        return Err(ManifestError::CheckpointSizeMismatch {
            expected: manifest.size_bytes,
            actual: metadata.len(),
        });
    }
    let checkpoint = fs::canonicalize(checkpoint)?;
    if !checkpoint.starts_with(manifest_dir) {
        return Err(ManifestError::UnsafeCheckpointPath);
    }
    Ok(ModelDescriptor {
        model_id: manifest.model_id,
        version: manifest.version,
        task: manifest.task,
        architecture: manifest.architecture,
        format,
        manifest_path,
        checkpoint,
        file_size_bytes: metadata.len(),
        input_name: manifest.input.name,
        input_data_type: manifest.input.data_type,
        input_shape: shape.clone(),
        output_schema: manifest.outputs.schema,
        output_name: manifest.outputs.name,
        output_data_type: manifest.outputs.data_type,
        output_shape: manifest.outputs.shape,
        expected_sha256: expected,
        license: manifest.license,
        source_url: manifest.source_url,
        validation: manifest.validation,
    })
}

/// Verify manifest metadata and stream the complete checkpoint through SHA-256.
pub fn verify_manifest(manifest_path: &Path) -> Result<VerifiedModel, ManifestError> {
    let descriptor = inspect_manifest(manifest_path)?;
    let actual = sha256_file(&descriptor.checkpoint)?;
    if actual != descriptor.expected_sha256 {
        return Err(ManifestError::CheckpointHashMismatch {
            expected: descriptor.expected_sha256,
            actual,
        });
    }
    Ok(VerifiedModel {
        sha256: actual,
        descriptor,
    })
}

fn is_placeholder(value: &str) -> bool {
    let value = value.trim();
    value.is_empty()
        || value.starts_with("replace-with")
        || value == "verify-before-use"
        || value.contains("example.invalid")
}

fn sha256_file(path: &Path) -> Result<String, ManifestError> {
    let file = fs::File::open(path).map_err(|_| ManifestError::CheckpointMissing(path.into()))?;
    let mut reader = io::BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| ManifestError::CheckpointMissing(path.into()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const CHECKPOINT_BYTES: &[u8] = b"hierarchiretina-fixture-weights\n";

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("retinacare-manifest-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn fixture_manifest() -> Value {
        json!({
            "schema_version": 1,
            "model_id": "hierarchiretina-stage1-gate",
            "version": "1.0.0",
            "task": "dr_screening",
            "architecture": "convnext_v2_l",
            "checkpoint": "gate.onnx",
            "size_bytes": CHECKPOINT_BYTES.len(),
            "sha256": hex::encode(Sha256::digest(CHECKPOINT_BYTES)),
            "license": "CC-BY-NC-4.0",
            "source_url": "https://github.com/asifuddin01/HierarchiRetina",
            "input": {
                "name": "image",
                "data_type": "float32",
                "shape": [1, 3, 768, 768],
                "color_space": "RGB",
                "normalization": "imagenet"
            },
            "outputs": {
                "schema": "binary_screening",
                "name": "logit",
                "data_type": "float32",
                "shape": [1, 1]
            },
            "validation": {"report": "docs/features/P0-01-hierarchiretina-model-loading"}
        })
    }

    fn write_fixture(directory: &Path, manifest: Value) -> PathBuf {
        fs::write(directory.join("gate.onnx"), CHECKPOINT_BYTES).unwrap();
        let path = directory.join("manifest.json");
        fs::write(&path, manifest.to_string()).unwrap();
        path
    }

    #[test]
    fn verifies_manifest_and_resolves_checkpoint() {
        let directory = scratch("verify");
        let path = write_fixture(&directory, fixture_manifest());
        let verified = verify_manifest(&path).unwrap();
        assert_eq!(verified.model_id, "hierarchiretina-stage1-gate");
        assert_eq!(verified.architecture, "convnext_v2_l");
        assert_eq!(verified.input_shape, vec![1, 3, 768, 768]);
        assert_eq!(verified.checkpoint, directory.join("gate.onnx"));
        assert_eq!(verified.format, ArtifactFormat::Onnx);
        assert_eq!(verified.file_size_bytes, CHECKPOINT_BYTES.len() as u64);
        assert_eq!(verified.sha256, verified.expected_sha256);
        assert_eq!(
            verified
                .validation
                .as_ref()
                .and_then(|value| value.get("report"))
                .and_then(serde_json::Value::as_str),
            Some("docs/features/P0-01-hierarchiretina-model-loading")
        );
    }

    #[test]
    fn inspects_pytorch_weights_as_conversion_required_artifacts() {
        let directory = scratch("pytorch");
        let mut manifest = fixture_manifest();
        manifest["checkpoint"] = json!("gate.pth");
        let path = write_fixture(&directory, manifest);
        fs::rename(directory.join("gate.onnx"), directory.join("gate.pth")).unwrap();
        assert_eq!(
            inspect_manifest(&path).unwrap().format,
            ArtifactFormat::PyTorch
        );
    }

    #[test]
    fn rejects_a_tampered_checkpoint() {
        let directory = scratch("tampered");
        let path = write_fixture(&directory, fixture_manifest());
        fs::write(directory.join("gate.onnx"), b"altered weights\n").unwrap();
        assert!(matches!(
            verify_manifest(&path),
            Err(ManifestError::CheckpointSizeMismatch { .. })
                | Err(ManifestError::CheckpointHashMismatch { .. })
        ));
    }

    #[test]
    fn rejects_a_missing_checkpoint() {
        let directory = scratch("missing-checkpoint");
        let path = write_fixture(&directory, fixture_manifest());
        fs::remove_file(directory.join("gate.onnx")).unwrap();
        assert!(matches!(
            inspect_manifest(&path),
            Err(ManifestError::CheckpointMissing(_))
        ));
    }

    #[test]
    fn rejects_the_placeholder_example_manifest() {
        let example =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../models/manifest.example.json");
        assert!(matches!(
            verify_manifest(&example),
            Err(ManifestError::InvalidField(_))
        ));
    }

    #[test]
    fn rejects_a_missing_required_field() {
        let directory = scratch("missing-field");
        let mut manifest = fixture_manifest();
        manifest.as_object_mut().unwrap().remove("sha256");
        let path = write_fixture(&directory, manifest);
        let error = verify_manifest(&path).unwrap_err().to_string();
        assert!(error.contains("sha256"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_an_unsupported_schema_version() {
        let directory = scratch("schema");
        let mut manifest = fixture_manifest();
        manifest["schema_version"] = json!(99);
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            verify_manifest(&path),
            Err(ManifestError::UnsupportedSchema(99))
        ));
    }

    #[test]
    fn rejects_a_malformed_sha256() {
        let directory = scratch("sha-format");
        let mut manifest = fixture_manifest();
        manifest["sha256"] = json!("not-a-checksum");
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            verify_manifest(&path),
            Err(ManifestError::InvalidField("sha256"))
        ));
    }

    #[test]
    fn rejects_an_input_shape_that_is_not_nchw_batch_one() {
        let directory = scratch("shape");
        let mut manifest = fixture_manifest();
        manifest["input"]["shape"] = json!([3, 768, 768]);
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            verify_manifest(&path),
            Err(ManifestError::InvalidField("input.shape"))
        ));
    }

    #[test]
    fn rejects_checkpoint_path_traversal() {
        let directory = scratch("traversal");
        let mut manifest = fixture_manifest();
        manifest["checkpoint"] = json!("../outside.onnx");
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            inspect_manifest(&path),
            Err(ManifestError::UnsafeCheckpointPath)
        ));
    }

    #[test]
    fn rejects_checkpoint_size_mismatch() {
        let directory = scratch("size-mismatch");
        let mut manifest = fixture_manifest();
        manifest["size_bytes"] = json!(1);
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            inspect_manifest(&path),
            Err(ManifestError::CheckpointSizeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_checkpoint_extensions() {
        let directory = scratch("extension");
        let mut manifest = fixture_manifest();
        manifest["checkpoint"] = json!("gate.bin");
        let path = write_fixture(&directory, manifest);
        assert!(matches!(
            inspect_manifest(&path),
            Err(ManifestError::UnsupportedArtifactFormat)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_checkpoint() {
        use std::os::unix::fs::symlink;
        let directory = scratch("symlink");
        let outside = directory.parent().unwrap().join("retinacare-outside.onnx");
        fs::write(&outside, CHECKPOINT_BYTES).unwrap();
        let path = write_fixture(&directory, fixture_manifest());
        fs::remove_file(directory.join("gate.onnx")).unwrap();
        symlink(&outside, directory.join("gate.onnx")).unwrap();
        assert!(matches!(
            inspect_manifest(&path),
            Err(ManifestError::CheckpointNotRegular(_))
        ));
        fs::remove_file(outside).unwrap();
    }
}
