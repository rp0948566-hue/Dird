//! Nonblocking application interface to the native Rust model runtime.
use retinacare_ai_bridge::{
    device::ProviderRequest,
    model_registry::{ModelCatalogEntry, ModelRegistry},
    model_runtime::{LoadedModel, ModelRuntime, Stage1GateOutput},
    stage2_contract::{EyeLaterality, Stage2MaskResult, Stage2ModelKind, Stage2Request},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelEvent {
    Catalog(Result<Vec<ModelCatalogEntry>, String>),
    Loaded(Result<LoadedModel, String>),
    Unloaded {
        model_id: String,
        removed: bool,
    },
    Inference {
        model_id: String,
        output: Result<Vec<f32>, String>,
    },
    Stage1Inference {
        model_id: String,
        result: Result<Stage1GateOutput, String>,
    },
    Stage2Inference {
        model_id: String,
        result: Result<Stage2MaskResult, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSubmitError {
    QueueFull,
    Stopped,
}

enum ModelCommand {
    Discover(PathBuf),
    Load(PathBuf, ProviderRequest),
    Unload(String),
    Infer(String, Vec<f32>),
    InferStage1(String, Vec<f32>),
    InferStage1Image(String, PathBuf),
    InferStage2Image(Stage2Request),
    Stop,
}

/// Serializes expensive file hashing, ONNX loading, inference and unload work
/// away from the GUI thread. `try_*` methods never wait for queue capacity.
pub struct ModelService {
    commands: SyncSender<ModelCommand>,
    events: Receiver<ModelEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ModelService {
    pub fn new() -> Self {
        let (commands, receiver) = mpsc::sync_channel(2);
        let (sender, events) = mpsc::sync_channel(4);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = thread::spawn(move || run_model_service(receiver, sender, thread_stop));
        Self {
            commands,
            events,
            stop,
            thread: Some(thread),
        }
    }

    pub fn discover(&self, root: PathBuf) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::Discover(root))
    }

    pub fn load(
        &self,
        manifest: PathBuf,
        provider: ProviderRequest,
    ) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::Load(manifest, provider))
    }

    pub fn unload(&self, model_id: String) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::Unload(model_id))
    }

    pub fn infer(&self, model_id: String, input: Vec<f32>) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::Infer(model_id, input))
    }

    pub fn infer_stage1(&self, model_id: String, input: Vec<f32>) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::InferStage1(model_id, input))
    }

    pub fn infer_stage1_image(
        &self,
        model_id: String,
        image_path: PathBuf,
    ) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::InferStage1Image(model_id, image_path))
    }

    pub fn infer_stage2_lesion_image(
        &self,
        model_id: String,
        examination_id: i64,
        image_path: PathBuf,
        model_kind: Stage2ModelKind,
        laterality: Option<EyeLaterality>,
    ) -> Result<(), ModelSubmitError> {
        self.submit(ModelCommand::InferStage2Image(Stage2Request {
            examination_id,
            image_path,
            laterality,
            model_kind,
            model_id,
        }))
    }

    pub fn try_next(&self) -> Result<ModelEvent, TryRecvError> {
        self.events.try_recv()
    }

    fn submit(&self, command: ModelCommand) -> Result<(), ModelSubmitError> {
        if self.stop.load(Ordering::Relaxed) {
            return Err(ModelSubmitError::Stopped);
        }
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) => ModelSubmitError::QueueFull,
                TrySendError::Disconnected(_) => ModelSubmitError::Stopped,
            })
    }
}

impl Default for ModelService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ModelService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.commands.try_send(ModelCommand::Stop);
        if let Some(thread) = self.thread.take() {
            while !thread.is_finished() {
                while self.events.try_recv().is_ok() {}
                thread::sleep(Duration::from_millis(5));
            }
            let _ = thread.join();
        }
    }
}

fn run_model_service(
    commands: Receiver<ModelCommand>,
    events: SyncSender<ModelEvent>,
    stop: Arc<AtomicBool>,
) {
    let mut runtime = ModelRuntime::default();
    while !stop.load(Ordering::Relaxed) {
        let Ok(command) = commands.recv() else {
            break;
        };
        let event = match command {
            ModelCommand::Discover(root) => ModelEvent::Catalog(
                ModelRegistry::discover(&root)
                    .map(|registry| registry.entries)
                    .map_err(|error| error.to_string()),
            ),
            ModelCommand::Load(path, provider) => {
                ModelEvent::Loaded(runtime.load(&path, provider).map_err(|e| e.to_string()))
            }
            ModelCommand::Unload(model_id) => ModelEvent::Unloaded {
                removed: runtime.unload(&model_id),
                model_id,
            },
            ModelCommand::Infer(model_id, input) => ModelEvent::Inference {
                output: runtime
                    .infer(&model_id, &input)
                    .map_err(|error| error.to_string()),
                model_id,
            },
            ModelCommand::InferStage1(model_id, input) => ModelEvent::Stage1Inference {
                result: runtime
                    .infer_stage1_gate(&model_id, &input)
                    .map_err(|error| error.to_string()),
                model_id,
            },
            ModelCommand::InferStage1Image(model_id, path) => ModelEvent::Stage1Inference {
                result: runtime
                    .infer_stage1_image(&model_id, &path)
                    .map_err(|error| error.to_string()),
                model_id,
            },
            ModelCommand::InferStage2Image(request) => ModelEvent::Stage2Inference {
                model_id: request.model_id.clone(),
                result: runtime
                    .infer_stage2_image(&request)
                    .map_err(|error| error.to_string()),
            },
            ModelCommand::Stop => break,
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn model_catalog_scan_completes_off_the_calling_thread() {
        let service = ModelService::new();
        let missing = std::env::temp_dir().join(format!(
            "retinacare-app-models-missing-{}",
            std::process::id()
        ));
        assert_eq!(service.discover(missing), Ok(()));
        let started = Instant::now();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Catalog(Ok(entries))) => {
                    assert!(entries.is_empty());
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(started.elapsed() < Duration::from_secs(2));
                    thread::yield_now();
                }
            }
        }
    }

    #[test]
    fn stage1_inference_reports_missing_model_as_a_typed_event() {
        let service = ModelService::new();
        assert_eq!(service.infer_stage1("missing".into(), vec![]), Ok(()));
        let started = Instant::now();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Stage1Inference { model_id, result }) => {
                    assert_eq!(model_id, "missing");
                    assert!(result.unwrap_err().contains("not loaded"));
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(started.elapsed() < Duration::from_secs(2));
                    thread::yield_now();
                }
            }
        }
    }

    #[test]
    fn stage1_image_inference_is_queued_and_returns_a_typed_error() {
        let service = ModelService::new();
        assert_eq!(
            service
                .infer_stage1_image("missing".into(), PathBuf::from("missing-retinal-image.png")),
            Ok(())
        );
        let started = Instant::now();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Stage1Inference { model_id, result }) => {
                    assert_eq!(model_id, "missing");
                    assert!(result.unwrap_err().contains("not loaded"));
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(started.elapsed() < Duration::from_secs(2));
                    thread::yield_now();
                }
            }
        }
    }

    #[test]
    fn stage2_ma_inference_is_queued_off_thread_and_reports_missing_model() {
        let service = ModelService::new();
        assert_eq!(
            service.infer_stage2_lesion_image(
                "missing-ma".into(),
                7,
                PathBuf::from("missing-image.png"),
                Stage2ModelKind::Ma,
                None,
            ),
            Ok(())
        );
        let started = Instant::now();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Stage2Inference { model_id, result }) => {
                    assert_eq!(model_id, "missing-ma");
                    assert!(result.unwrap_err().contains("not loaded"));
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(started.elapsed() < Duration::from_secs(2));
                    thread::yield_now();
                }
            }
        }
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE1_IMAGE to the user-approved test image"]
    fn local_stage1_image_runs_through_the_nonblocking_model_service() {
        let image = PathBuf::from(
            std::env::var_os("RETINACARE_STAGE1_IMAGE")
                .expect("set RETINACARE_STAGE1_IMAGE to a local retinal image"),
        );
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../models/stage1_convnextv2l_768/manifest.json");
        let provider = match std::env::var("RETINACARE_STAGE1_PROVIDER").as_deref() {
            Ok("cuda") => ProviderRequest::Cuda { device_id: 0 },
            Ok("cpu") | Err(_) => ProviderRequest::Cpu,
            Ok(other) => panic!("unsupported RETINACARE_STAGE1_PROVIDER: {other}"),
        };
        let service = ModelService::new();
        service.load(manifest, provider).unwrap();
        let deadline = Instant::now() + Duration::from_secs(90);
        let model_id = loop {
            match service.try_next() {
                Ok(ModelEvent::Loaded(Ok(model))) => break model.model_id,
                Ok(ModelEvent::Loaded(Err(error))) => panic!("model load failed: {error}"),
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "model load timed out");
                    thread::yield_now();
                }
            }
        };
        service.infer_stage1_image(model_id.clone(), image).unwrap();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Stage1Inference {
                    model_id: event_model,
                    result,
                }) => {
                    assert_eq!(event_model, model_id);
                    let output = result.expect("Stage I image inference failed");
                    let expected_provider = match provider {
                        ProviderRequest::Cpu => retinacare_ai_bridge::device::ActualProvider::Cpu,
                        ProviderRequest::Cuda { device_id } => {
                            retinacare_ai_bridge::device::ActualProvider::Cuda { device_id }
                        }
                    };
                    assert_eq!(output.provider, expected_provider);
                    let image = output.image.expect("image provenance metadata missing");
                    assert_eq!(image.width, 640);
                    assert_eq!(image.height, 640);
                    assert_eq!(image.sha256.len(), 64);
                    assert_eq!(
                        output.preprocessing_version,
                        "hierarchiretina-v1.0-preprocess_gate"
                    );
                    assert!(output.preprocessing_microseconds.unwrap() > 0);
                    assert!(output.elapsed_microseconds > 0);
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "image inference timed out");
                    thread::yield_now();
                }
            }
        }
    }

    #[test]
    #[ignore = "local-only: set Stage II MA manifest, image and parity fixture paths"]
    fn local_stage2_ma_runs_through_nonblocking_service_and_matches_reference_mask() {
        verify_local_stage2_lesion_service(Stage2ModelKind::Ma, "MA");
    }

    #[test]
    #[ignore = "local-only: set Stage II HE manifest, image and parity fixture paths"]
    fn local_stage2_he_runs_through_nonblocking_service_and_matches_reference_mask() {
        verify_local_stage2_lesion_service(Stage2ModelKind::He, "HE");
    }

    #[test]
    #[ignore = "local-only: set Stage II EX manifest, image and parity fixture paths"]
    fn local_stage2_ex_runs_through_nonblocking_service_and_matches_reference_mask() {
        verify_local_stage2_lesion_service(Stage2ModelKind::Ex, "EX");
    }

    #[test]
    #[ignore = "local-only: set Stage II CWS manifest, image and parity fixture paths"]
    fn local_stage2_cws_runs_through_nonblocking_service_and_matches_reference_mask() {
        verify_local_stage2_lesion_service(Stage2ModelKind::Cws, "CWS");
    }

    #[test]
    #[ignore = "local-only: set Stage II VESSEL manifest, image and parity fixture paths"]
    fn local_stage2_vessel_runs_through_nonblocking_service_and_matches_reference_mask() {
        verify_local_stage2_lesion_service(Stage2ModelKind::Vessel, "VESSEL");
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_{MA,HE,EX,CWS,VESSEL}_{MANIFEST,PARITY_DIR} and _IMAGE"]
    fn local_stage2_five_masks_run_sequentially_in_fixed_order() {
        // Models are loaded, run and unloaded one at a time to bound CPU/GPU memory.
        let image = PathBuf::from(
            std::env::var_os("RETINACARE_STAGE2_MA_IMAGE")
                .expect("set RETINACARE_STAGE2_MA_IMAGE to the local technical test image"),
        );
        let service = ModelService::new();
        let order = [
            (Stage2ModelKind::Ma, "MA"),
            (Stage2ModelKind::He, "HE"),
            (Stage2ModelKind::Ex, "EX"),
            (Stage2ModelKind::Cws, "CWS"),
            (Stage2ModelKind::Vessel, "VESSEL"),
        ];
        let mut source_sha: Option<String> = None;
        for (model_kind, model_code) in order {
            let manifest = PathBuf::from(
                std::env::var_os(format!("RETINACARE_STAGE2_{model_code}_MANIFEST"))
                    .expect("set lesion or vessel manifest environment variable"),
            );
            let parity_dir = PathBuf::from(
                std::env::var_os(format!("RETINACARE_STAGE2_{model_code}_PARITY_DIR"))
                    .expect("set parity directory environment variable"),
            );
            service.load(manifest, ProviderRequest::Cpu).unwrap();
            let deadline = Instant::now() + Duration::from_secs(180);
            let model_id = loop {
                match service.try_next() {
                    Ok(ModelEvent::Loaded(Ok(model))) => break model.model_id,
                    Ok(ModelEvent::Loaded(Err(error))) => {
                        panic!("{model_code} load failed: {error}")
                    }
                    Ok(other) => panic!("unexpected event before {model_code} load: {other:?}"),
                    Err(TryRecvError::Disconnected) => panic!("model service disconnected"),
                    Err(TryRecvError::Empty) => {
                        assert!(Instant::now() < deadline, "{model_code} load timed out");
                        thread::yield_now();
                    }
                }
            };
            service
                .infer_stage2_lesion_image(model_id.clone(), 7, image.clone(), model_kind, None)
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(600);
            let output = loop {
                match service.try_next() {
                    Ok(ModelEvent::Stage2Inference {
                        model_id: event_model,
                        result,
                    }) => {
                        assert_eq!(event_model, model_id);
                        break result.unwrap_or_else(|error| {
                            panic!("Stage II {model_code} inference failed: {error}")
                        });
                    }
                    Ok(other) => panic!("unexpected event during {model_code}: {other:?}"),
                    Err(TryRecvError::Disconnected) => panic!("model service disconnected"),
                    Err(TryRecvError::Empty) => {
                        assert!(
                            Instant::now() < deadline,
                            "{model_code} inference timed out"
                        );
                        thread::yield_now();
                    }
                }
            };
            output.validate().unwrap();
            assert_eq!(output.model_kind, model_kind);
            assert_eq!(output.examination_id, 7);
            match &source_sha {
                None => source_sha = Some(output.source_image_sha256.clone()),
                Some(sha) => {
                    assert_eq!(&output.source_image_sha256, sha, "same image for all masks")
                }
            }
            let retinacare_ai_bridge::stage2_contract::MaskPixelData::Binary(mask) = output.data
            else {
                panic!("expected binary Stage II {model_code} mask");
            };
            let reference = std::fs::read(parity_dir.join(format!(
                "{}_pytorch_mask.u8",
                model_code.to_ascii_lowercase()
            )))
            .unwrap();
            assert_eq!(mask, reference, "Rust/PyTorch {model_code} mask mismatch");
            service.unload(model_id.clone()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                match service.try_next() {
                    Ok(ModelEvent::Unloaded {
                        model_id: event_model,
                        removed,
                    }) => {
                        assert_eq!(event_model, model_id);
                        assert!(removed, "{model_code} session was not released");
                        break;
                    }
                    Ok(other) => panic!("unexpected event after {model_code}: {other:?}"),
                    Err(TryRecvError::Disconnected) => panic!("model service disconnected"),
                    Err(TryRecvError::Empty) => {
                        assert!(Instant::now() < deadline, "{model_code} unload timed out");
                        thread::yield_now();
                    }
                }
            }
        }
    }

    fn verify_local_stage2_lesion_service(model_kind: Stage2ModelKind, model_code: &str) {
        let prefix = model_code.to_ascii_lowercase();
        let manifest = PathBuf::from(
            std::env::var_os(format!("RETINACARE_STAGE2_{model_code}_MANIFEST"))
                .expect("set lesion manifest environment variable"),
        );
        let image = PathBuf::from(
            std::env::var_os(format!("RETINACARE_STAGE2_{model_code}_IMAGE"))
                .expect("set lesion image environment variable"),
        );
        let parity_dir = PathBuf::from(
            std::env::var_os(format!("RETINACARE_STAGE2_{model_code}_PARITY_DIR"))
                .expect("set lesion parity directory environment variable"),
        );
        let service = ModelService::new();
        service.load(manifest, ProviderRequest::Cpu).unwrap();
        let deadline = Instant::now() + Duration::from_secs(120);
        let model_id = loop {
            match service.try_next() {
                Ok(ModelEvent::Loaded(Ok(model))) => break model.model_id,
                Ok(ModelEvent::Loaded(Err(error))) => panic!("model load failed: {error}"),
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("model service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(Instant::now() < deadline, "model load timed out");
                    thread::yield_now();
                }
            }
        };
        service
            .infer_stage2_lesion_image(model_id.clone(), 7, image, model_kind, None)
            .unwrap();
        loop {
            match service.try_next() {
                Ok(ModelEvent::Stage2Inference {
                    model_id: event_model,
                    result,
                }) => {
                    assert_eq!(event_model, model_id);
                    let output = result.unwrap_or_else(|error| {
                        panic!("Stage II {model_code} inference failed: {error}")
                    });
                    output.validate().unwrap();
                    assert_eq!(output.model_kind, model_kind);
                    let retinacare_ai_bridge::stage2_contract::Stage2MaskResult {
                        provider,
                        elapsed_microseconds,
                        data,
                        ..
                    } = output;
                    assert_eq!(provider, retinacare_ai_bridge::device::ActualProvider::Cpu);
                    assert!(elapsed_microseconds > 0);
                    let retinacare_ai_bridge::stage2_contract::MaskPixelData::Binary(mask) = data
                    else {
                        panic!("expected binary Stage II {model_code} mask");
                    };
                    let reference =
                        std::fs::read(parity_dir.join(format!("{prefix}_pytorch_mask.u8")))
                            .unwrap();
                    assert_eq!(
                        mask, reference,
                        "nonblocking Rust/PyTorch {model_code} mask mismatch"
                    );
                    break;
                }
                Ok(other) => panic!("unexpected event: {other:?}"),
                Err(TryRecvError::Disconnected) => panic!("model service disconnected"),
                Err(TryRecvError::Empty) => {
                    assert!(
                        Instant::now() < deadline,
                        "{model_code} inference timed out"
                    );
                    thread::yield_now();
                }
            }
        }
    }
}
