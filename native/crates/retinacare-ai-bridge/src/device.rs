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
