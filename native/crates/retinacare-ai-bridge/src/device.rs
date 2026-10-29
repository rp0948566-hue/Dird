//! Explicit execution-provider requests and capability reporting.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRequest {
    Cpu,
    Cuda { device_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActualProvider {
    Cpu,
    Cuda { device_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub cpu: bool,
    /// Means the linked ONNX Runtime exposes its CUDA EP; it does not prove a
    /// compatible driver/GPU is present. Successful CUDA session creation is
    /// the runtime check.
    pub cuda_execution_provider: bool,
}

/// What to do with a model-load request that asks for CPU (`RETINACARE_PROVIDER`).
/// `cpu` keeps it there, `cuda` moves it to GPU 0 and fails loudly, `auto` tries GPU 0 and
/// falls back to CPU. Unset means `auto` in a CUDA build and `cpu` otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPolicy {
    Cpu,
    Auto,
    Cuda,
}

pub fn gpu_policy() -> GpuPolicy {
    match std::env::var("RETINACARE_PROVIDER").as_deref() {
        Ok("cpu") => GpuPolicy::Cpu,
        Ok("cuda") => GpuPolicy::Cuda,
        Ok("auto") => GpuPolicy::Auto,
        _ if cfg!(feature = "cuda") => GpuPolicy::Auto,
        _ => GpuPolicy::Cpu,
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("ONNX Runtime CUDA capability probe failed: {0}")]
    Probe(String),
}

pub fn provider_capabilities() -> Result<ProviderCapabilities, DeviceError> {
    #[cfg(feature = "cuda")]
    let cuda_execution_provider = {
        use ort::ep::ExecutionProvider;
        ort::ep::CUDA::is_available().map_err(|error| DeviceError::Probe(error.to_string()))?
    };

    #[cfg(not(feature = "cuda"))]
    let cuda_execution_provider = false;

    Ok(ProviderCapabilities {
        cpu: true,
        cuda_execution_provider,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_is_always_reported_and_cuda_is_never_guessed() {
        let capabilities = provider_capabilities().unwrap();
        assert!(capabilities.cpu);
        #[cfg(not(feature = "cuda"))]
        assert!(!capabilities.cuda_execution_provider);
    }
}
