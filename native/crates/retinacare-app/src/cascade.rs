//! M5A: nonblocking Stage I -> Stage II (five masks) -> Stage III (LG-DRG fold 0) cascade.
//!
//! Routing follows pinned HierarchiRetina v1.0 (`stage3/cascade.py`): a Stage I probability below
//! the gate threshold is a negative route (cascade grade 0, Stages II and III are not run); at or
//! above it the five Stage II masks are produced and Stage III grades the eight-channel input, with
//! grade 5 meaning ungradable. A failed stage is an error, never a grade. Results are a SINGLE-FOLD
//! TECHNICAL TEST, research only.
use retinacare_ai_bridge::{
    device::{ActualProvider, ProviderRequest},
    model_runtime::{ModelRuntime, ModelRuntimeError, Stage1GateOutput, Stage1InferenceError},
    onnx_session::SessionError,
    stage1::Stage1GateDecision,
    stage2_contract::{
        EyeLaterality, MaskPixelData, Stage2InferenceError, Stage2MaskResult, Stage2ModelKind,
        Stage2Request,
    },
    stage3_grading::{Stage3GradeResult, Stage3InferenceError, Stage3Request, Stage3Scope},
    stage3_input::{STAGE3_MASK_ORDER, Stage3InputError},
};
use serde::Serialize;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    time::Instant,
};

pub type RequestId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStage {
    Screening,
    Segmentation(Stage2ModelKind),
    Grading,
}

/// Only the single-fold technical research policy exists until the five-fold ensemble is verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPolicy {
    TechnicalFold0,
}

#[derive(Debug, Clone)]
pub struct AnalysisRequest {
    pub request_id: RequestId,
    pub examination_id: i64,
    pub patient_id: Option<i64>,
    pub eye: Option<EyeLaterality>,
    pub image_path: PathBuf,
    pub policy: AnalysisPolicy,
}

/// Manifest paths of the seven ONNX models the cascade loads one at a time.
#[derive(Debug, Clone)]
pub struct CascadeModelPaths {
    pub stage1: PathBuf,
    pub ma: PathBuf,
    pub he: PathBuf,
    pub ex: PathBuf,
    pub cws: PathBuf,
    pub vessel: PathBuf,
    pub stage3_fold0: PathBuf,
}

impl CascadeModelPaths {
    fn segmentation(&self, kind: Stage2ModelKind) -> &Path {
        match kind {
            Stage2ModelKind::Ma => &self.ma,
            Stage2ModelKind::He => &self.he,
            Stage2ModelKind::Ex => &self.ex,
            Stage2ModelKind::Cws => &self.cws,
            Stage2ModelKind::Vessel => &self.vessel,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageFailureKind {
    InvalidRequest,
    InvalidSourceImage,
    ModelUnavailable,
    ModelIntegrity,
    ModelMismatch,
    InvalidMasks,
    IdentityMismatch,
    Inference,
    InvalidOutput,
    Internal,
}

impl StageFailureKind {
    /// Fixed text: no paths, patient identifiers or tensor values.
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "the analysis request is invalid",
            Self::InvalidSourceImage => "the source image could not be validated",
            Self::ModelUnavailable => "a required model is missing or could not be loaded",
            Self::ModelIntegrity => "a model artifact failed its integrity check",
            Self::ModelMismatch => "a loaded model does not match the expected contract",
            Self::InvalidMasks => "the Stage II masks are missing, duplicated or malformed",
            Self::IdentityMismatch => "a stage result does not belong to this image or examination",
            Self::Inference => "model inference failed",
            Self::InvalidOutput => "a model output was invalid",
            Self::Internal => "an internal error interrupted the stage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StageFailure {
    pub stage: AnalysisStage,
    pub kind: StageFailureKind,
    pub message: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisState {
    Graded,
    Ungradable,
    NegativeRoute,
    Cancelled,
    Failed,
}

/// The cascade-level grade, never a bare number: grade 0 only comes from the Stage I gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CascadeGrade {
    Grade0FromStage1Gate,
    Severity(u8),
    Ungradable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    NegativeScreeningRoute,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScreeningSummary {
    pub decision: Stage1GateDecision,
    pub screening_score: f32,
    pub threshold: f32,
    pub provider: ActualProvider,
    pub elapsed_microseconds: u128,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaskSummary {
    pub kind: Stage2ModelKind,
    pub width: u32,
    pub height: u32,
    pub foreground_pixels: u64,
    pub threshold: f32,
    pub model_sha256: String,
    pub checkpoint_sha256: String,
    pub provider: ActualProvider,
    pub elapsed_microseconds: u128,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GradeSummary {
    pub grade: u8,
    pub ungradable: bool,
    pub scope: Stage3Scope,
    pub provider: ActualProvider,
    pub elapsed_microseconds: u128,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum StageSummary {
    Screening(ScreeningSummary),
    Segmentation(MaskSummary),
    Grading(GradeSummary),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StageTiming {
    pub stage: AnalysisStage,
    pub wall_microseconds: u128,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalysisReport {
    pub request_id: RequestId,
    pub examination_id: i64,
    pub patient_id: Option<i64>,
    pub eye: Option<EyeLaterality>,
    pub state: AnalysisState,
    pub cascade_grade: Option<CascadeGrade>,
    pub source_image_sha256: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub screening: Option<Stage1GateOutput>,
    pub masks: Vec<Stage2MaskResult>,
    pub grading: Option<Stage3GradeResult>,
    pub failure: Option<StageFailure>,
    pub cancelled_before: Option<AnalysisStage>,
    /// True when the run ended early: any stage output kept is diagnostic evidence, not a result.
    pub evidence_is_diagnostic_only: bool,
    pub timings: Vec<StageTiming>,
    pub total_microseconds: u128,
    pub peak_rss_kib: Option<u64>,
    pub policy: AnalysisPolicy,
    pub research_only: bool,
    pub origin: &'static str,
    pub validation_status: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisEvent {
    Started {
        request_id: RequestId,
    },
    StageStarted {
        request_id: RequestId,
        stage: AnalysisStage,
    },
    StageCompleted {
        request_id: RequestId,
        stage: AnalysisStage,
        summary: StageSummary,
    },
    StageSkipped {
        request_id: RequestId,
        stage: AnalysisStage,
        reason: SkipReason,
    },
    /// Exactly one per accepted request.
    Finished(Box<AnalysisReport>),
}

/// The three kinds of model work the state machine needs. Real inference lives behind this trait so
/// the routing can be tested deterministically without weights.
pub trait CascadeRunner {
    fn screening(
        &mut self,
        request: &AnalysisRequest,
    ) -> Result<Stage1GateOutput, StageFailureKind>;
    fn segment(
        &mut self,
        request: &AnalysisRequest,
        kind: Stage2ModelKind,
    ) -> Result<Stage2MaskResult, StageFailureKind>;
    fn grade(
        &mut self,
        request: &AnalysisRequest,
        masks: &[Stage2MaskResult],
    ) -> Result<Stage3GradeResult, StageFailureKind>;
}

struct Run {
    started: Instant,
    report: AnalysisReport,
}

impl Run {
    fn finish(mut self, state: AnalysisState) -> AnalysisReport {
        self.report.state = state;
        self.report.total_microseconds = self.started.elapsed().as_micros();
        self.report.peak_rss_kib = peak_rss_kib();
        self.report.evidence_is_diagnostic_only =
            matches!(state, AnalysisState::Failed | AnalysisState::Cancelled)
                && (self.report.screening.is_some() || !self.report.masks.is_empty());
        self.report
    }

    fn fail(mut self, stage: AnalysisStage, kind: StageFailureKind) -> AnalysisReport {
        self.report.failure = Some(StageFailure {
            stage,
            kind,
            message: kind.message(),
        });
        self.report.grading = None;
        self.report.cascade_grade = None;
        self.finish(AnalysisState::Failed)
    }

    fn cancel(mut self, before: AnalysisStage) -> AnalysisReport {
        self.report.cancelled_before = Some(before);
        self.report.grading = None;
        self.report.cascade_grade = None;
        self.finish(AnalysisState::Cancelled)
    }
}

/// Execute one analysis. Emits `Started`, per-stage events and exactly one `Finished` (also returned).
pub fn run_cascade(
    runner: &mut dyn CascadeRunner,
    request: &AnalysisRequest,
    is_cancelled: &dyn Fn() -> bool,
    emit: &mut dyn FnMut(AnalysisEvent),
) -> AnalysisReport {
    let id = request.request_id;
    let mut run = Run {
        started: Instant::now(),
        report: AnalysisReport {
            request_id: id,
            examination_id: request.examination_id,
            patient_id: request.patient_id,
            eye: request.eye,
            state: AnalysisState::Failed,
            cascade_grade: None,
            source_image_sha256: None,
            image_width: None,
            image_height: None,
            screening: None,
            masks: Vec::new(),
            grading: None,
            failure: None,
            cancelled_before: None,
            evidence_is_diagnostic_only: false,
            timings: Vec::new(),
            total_microseconds: 0,
            peak_rss_kib: None,
            policy: request.policy,
            research_only: true,
            origin: "official-model-inference",
            validation_status: "technical_only",
        },
    };
    emit(AnalysisEvent::Started { request_id: id });
    let terminal = |report: AnalysisReport, emit: &mut dyn FnMut(AnalysisEvent)| {
        emit(AnalysisEvent::Finished(Box::new(report.clone())));
        report
    };

    if request.examination_id <= 0 || request.request_id == 0 {
        let report = run.fail(AnalysisStage::Screening, StageFailureKind::InvalidRequest);
        return terminal(report, emit);
    }

    // Stage I: screening gate.
    if is_cancelled() {
        return terminal(run.cancel(AnalysisStage::Screening), emit);
    }
    emit(AnalysisEvent::StageStarted {
        request_id: id,
        stage: AnalysisStage::Screening,
    });
    let stage_started = Instant::now();
    let screening = match runner.screening(request) {
        Ok(output) => output,
        Err(kind) => return terminal(run.fail(AnalysisStage::Screening, kind), emit),
    };
    run.report.timings.push(StageTiming {
        stage: AnalysisStage::Screening,
        wall_microseconds: stage_started.elapsed().as_micros(),
    });
    let Some(image) = screening.image.clone() else {
        run.report.screening = Some(screening);
        return terminal(
            run.fail(AnalysisStage::Screening, StageFailureKind::InvalidOutput),
            emit,
        );
    };
    let (source_sha, width, height) = (image.sha256, image.width, image.height);
    run.report.source_image_sha256 = Some(source_sha.clone());
    run.report.image_width = Some(width);
    run.report.image_height = Some(height);
    let decision = screening.gate.decision;
    emit(AnalysisEvent::StageCompleted {
        request_id: id,
        stage: AnalysisStage::Screening,
        summary: StageSummary::Screening(ScreeningSummary {
            decision,
            screening_score: screening.gate.screening_score,
            threshold: screening.gate.threshold,
            provider: screening.provider,
            elapsed_microseconds: screening.elapsed_microseconds,
        }),
    });
    run.report.screening = Some(screening);

    if decision == Stage1GateDecision::NegativeRoute {
        for kind in STAGE3_MASK_ORDER {
            emit(AnalysisEvent::StageSkipped {
                request_id: id,
                stage: AnalysisStage::Segmentation(kind),
                reason: SkipReason::NegativeScreeningRoute,
            });
        }
        emit(AnalysisEvent::StageSkipped {
            request_id: id,
            stage: AnalysisStage::Grading,
            reason: SkipReason::NegativeScreeningRoute,
        });
        run.report.cascade_grade = Some(CascadeGrade::Grade0FromStage1Gate);
        return terminal(run.finish(AnalysisState::NegativeRoute), emit);
    }

    // Stage II: five masks, fixed order.
    for kind in STAGE3_MASK_ORDER {
        let stage = AnalysisStage::Segmentation(kind);
        if is_cancelled() {
            return terminal(run.cancel(stage), emit);
        }
        emit(AnalysisEvent::StageStarted {
            request_id: id,
            stage,
        });
        let stage_started = Instant::now();
        let mask = match runner.segment(request, kind) {
            Ok(mask) => mask,
            Err(failure) => return terminal(run.fail(stage, failure), emit),
        };
        run.report.timings.push(StageTiming {
            stage,
            wall_microseconds: stage_started.elapsed().as_micros(),
        });
        if let Err(kind_error) = validate_mask(&mask, kind, request, &source_sha, width, height) {
            return terminal(run.fail(stage, kind_error), emit);
        }
        emit(AnalysisEvent::StageCompleted {
            request_id: id,
            stage,
            summary: StageSummary::Segmentation(mask_summary(&mask)),
        });
        run.report.masks.push(mask);
    }

    // Stage III: fold 0 grading.
    if is_cancelled() {
        return terminal(run.cancel(AnalysisStage::Grading), emit);
    }
    emit(AnalysisEvent::StageStarted {
        request_id: id,
        stage: AnalysisStage::Grading,
    });
    let stage_started = Instant::now();
    let grading = match runner.grade(request, &run.report.masks) {
        Ok(result) => result,
        Err(failure) => return terminal(run.fail(AnalysisStage::Grading, failure), emit),
    };
    run.report.timings.push(StageTiming {
        stage: AnalysisStage::Grading,
        wall_microseconds: stage_started.elapsed().as_micros(),
    });
    let valid = grading.examination_id == request.examination_id
        && grading.source_image_sha256 == source_sha
        && grading.scope == (Stage3Scope::SingleFoldTechnicalTest { fold: 0 })
        && grading.research_only
        && (1..=5).contains(&grading.grade)
        && grading.ungradable == (grading.grade == 5);
    if !valid {
        return terminal(
            run.fail(AnalysisStage::Grading, StageFailureKind::IdentityMismatch),
            emit,
        );
    }
    emit(AnalysisEvent::StageCompleted {
        request_id: id,
        stage: AnalysisStage::Grading,
        summary: StageSummary::Grading(GradeSummary {
            grade: grading.grade,
            ungradable: grading.ungradable,
            scope: grading.scope,
            provider: grading.provider,
            elapsed_microseconds: grading.elapsed_microseconds,
        }),
    });
    let state = if grading.ungradable {
        run.report.cascade_grade = Some(CascadeGrade::Ungradable);
        AnalysisState::Ungradable
    } else {
        run.report.cascade_grade = Some(CascadeGrade::Severity(grading.grade));
        AnalysisState::Graded
    };
    run.report.grading = Some(grading);
    terminal(run.finish(state), emit)
}

fn validate_mask(
    mask: &Stage2MaskResult,
    kind: Stage2ModelKind,
    request: &AnalysisRequest,
    source_sha: &str,
    width: u32,
    height: u32,
) -> Result<(), StageFailureKind> {
    if mask.model_kind != kind || mask.validate().is_err() {
        return Err(StageFailureKind::InvalidMasks);
    }
    if mask.examination_id != request.examination_id
        || mask.source_image_sha256 != source_sha
        || mask.laterality != request.eye
    {
        return Err(StageFailureKind::IdentityMismatch);
    }
    if mask.original_width != width
        || mask.original_height != height
        || mask.mask_width != width
        || mask.mask_height != height
    {
        return Err(StageFailureKind::IdentityMismatch);
    }
    Ok(())
}

fn mask_summary(mask: &Stage2MaskResult) -> MaskSummary {
    let foreground_pixels = match &mask.data {
        MaskPixelData::Binary(pixels) => {
            pixels.iter().filter(|&&pixel| pixel == 255).count() as u64
        }
        MaskPixelData::Probability(_) => 0,
    };
    MaskSummary {
        kind: mask.model_kind,
        width: mask.mask_width,
        height: mask.mask_height,
        foreground_pixels,
        threshold: mask.threshold.value,
        model_sha256: mask.model_sha256.clone(),
        checkpoint_sha256: mask.checkpoint_sha256.clone(),
        provider: mask.provider,
        elapsed_microseconds: mask.elapsed_microseconds,
    }
}

/// Peak resident set size of this process in KiB (Linux `VmHWM`); `None` elsewhere.
pub fn peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.trim().trim_end_matches("kB").trim().parse().ok())
}

/// Real runner: loads each model on the CPU provider, runs it and unloads it again, so the six Stage I/II
/// models and fold 0 are never resident together. A model the caller already loaded is reused and left loaded.
pub struct ModelRuntimeRunner<'a> {
    runtime: &'a mut ModelRuntime,
    paths: &'a CascadeModelPaths,
}

impl<'a> ModelRuntimeRunner<'a> {
    pub fn new(runtime: &'a mut ModelRuntime, paths: &'a CascadeModelPaths) -> Self {
        Self { runtime, paths }
    }

    fn with_model<T>(
        &mut self,
        manifest: &Path,
        run: impl FnOnce(&mut ModelRuntime, &str) -> Result<T, StageFailureKind>,
    ) -> Result<T, StageFailureKind> {
        let (model_id, owned) = match self.runtime.load(manifest, ProviderRequest::Cpu) {
            Ok(loaded) => (loaded.model_id, true),
            Err(ModelRuntimeError::AlreadyLoaded(id)) => (id, false),
            Err(error) => return Err(classify_runtime(&error)),
        };
        let runtime = &mut *self.runtime;
        let outcome = catch_unwind(AssertUnwindSafe(|| run(runtime, &model_id)));
        if owned {
            self.runtime.unload(&model_id);
        }
        outcome.unwrap_or(Err(StageFailureKind::Internal))
    }
}

impl CascadeRunner for ModelRuntimeRunner<'_> {
    fn screening(
        &mut self,
        request: &AnalysisRequest,
    ) -> Result<Stage1GateOutput, StageFailureKind> {
        let manifest = self.paths.stage1.clone();
        self.with_model(&manifest, |runtime, model_id| {
            runtime
                .infer_stage1_image(model_id, &request.image_path)
                .map_err(|error| classify_stage1(&error))
        })
    }

    fn segment(
        &mut self,
        request: &AnalysisRequest,
        kind: Stage2ModelKind,
    ) -> Result<Stage2MaskResult, StageFailureKind> {
        let manifest = self.paths.segmentation(kind).to_path_buf();
        self.with_model(&manifest, |runtime, model_id| {
            runtime
                .infer_stage2_image(&Stage2Request {
                    examination_id: request.examination_id,
                    image_path: request.image_path.clone(),
                    laterality: request.eye,
                    model_kind: kind,
                    model_id: model_id.to_owned(),
                })
                .map_err(|error| classify_stage2(&error))
        })
    }

    fn grade(
        &mut self,
        request: &AnalysisRequest,
        masks: &[Stage2MaskResult],
    ) -> Result<Stage3GradeResult, StageFailureKind> {
        let manifest = self.paths.stage3_fold0.clone();
        self.with_model(&manifest, |runtime, model_id| {
            runtime
                .infer_stage3_fold(&Stage3Request {
                    examination_id: request.examination_id,
                    image_path: request.image_path.clone(),
                    masks: masks.to_vec(),
                    model_id: model_id.to_owned(),
                })
                .map_err(|error| classify_stage3(&error))
        })
    }
}

fn classify_session(error: &SessionError) -> StageFailureKind {
    use retinacare_ai_bridge::model_manifest::ManifestError;
    match error {
        SessionError::Manifest(
            ManifestError::CheckpointHashMismatch { .. }
            | ManifestError::CheckpointSizeMismatch { .. },
        ) => StageFailureKind::ModelIntegrity,
        SessionError::Manifest(_)
        | SessionError::Runtime(_)
        | SessionError::Build(_)
        | SessionError::ConversionRequired
        | SessionError::CudaNotCompiled
        | SessionError::InvalidCudaDevice(_) => StageFailureKind::ModelUnavailable,
        SessionError::InputMismatch(_) => StageFailureKind::ModelMismatch,
        SessionError::InputSize { .. } | SessionError::NonFiniteInput => {
            StageFailureKind::InvalidOutput
        }
        SessionError::Run(_) => StageFailureKind::Inference,
    }
}

fn classify_runtime(error: &ModelRuntimeError) -> StageFailureKind {
    match error {
        ModelRuntimeError::Session(session) => classify_session(session),
        ModelRuntimeError::AlreadyLoaded(_) | ModelRuntimeError::NotLoaded(_) => {
            StageFailureKind::ModelUnavailable
        }
    }
}

fn classify_stage1(error: &Stage1InferenceError) -> StageFailureKind {
    match error {
        Stage1InferenceError::Runtime(runtime) => classify_runtime(runtime),
        Stage1InferenceError::Session(session) => classify_session(session),
        Stage1InferenceError::IncompatibleModel(_) => StageFailureKind::ModelMismatch,
        Stage1InferenceError::Decode(_) => StageFailureKind::InvalidOutput,
        Stage1InferenceError::Preprocessing(_) => StageFailureKind::InvalidSourceImage,
    }
}

fn classify_stage2(error: &Stage2InferenceError) -> StageFailureKind {
    match error {
        Stage2InferenceError::Request(_) => StageFailureKind::InvalidRequest,
        Stage2InferenceError::Runtime(runtime) => classify_runtime(runtime),
        Stage2InferenceError::Session(session) => classify_session(session),
        Stage2InferenceError::Contract(_) | Stage2InferenceError::Decode(_) => {
            StageFailureKind::InvalidOutput
        }
        Stage2InferenceError::IncompatibleModel(_) => StageFailureKind::ModelMismatch,
        Stage2InferenceError::Preprocessing(_) => StageFailureKind::InvalidSourceImage,
    }
}

fn classify_stage3(error: &Stage3InferenceError) -> StageFailureKind {
    match error {
        Stage3InferenceError::InvalidRequest(_) => StageFailureKind::InvalidRequest,
        Stage3InferenceError::Runtime(runtime) => classify_runtime(runtime),
        Stage3InferenceError::Session(session) => classify_session(session),
        Stage3InferenceError::Input(input) => match input {
            Stage3InputError::Preprocess(_) | Stage3InputError::OpenCv(_) => {
                StageFailureKind::InvalidSourceImage
            }
            Stage3InputError::StaleExamination(_)
            | Stage3InputError::SourceMismatch(_)
            | Stage3InputError::DimensionMismatch(_) => StageFailureKind::IdentityMismatch,
            Stage3InputError::MissingMask(_)
            | Stage3InputError::DuplicateMask(_)
            | Stage3InputError::InvalidMaskResult(_)
            | Stage3InputError::NonBinaryMask(_) => StageFailureKind::InvalidMasks,
        },
        Stage3InferenceError::Source(_) => StageFailureKind::InvalidSourceImage,
        Stage3InferenceError::Grading(_) | Stage3InferenceError::BadOutput(_) => {
            StageFailureKind::InvalidOutput
        }
        Stage3InferenceError::IncompatibleModel => StageFailureKind::ModelMismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AnalysisSubmitError, ModelService};
    use retinacare_ai_bridge::{
        model_runtime::Stage1ImageMetadata,
        stage1::decode_stage1_gate,
        stage2_contract::{
            ImageRect, Stage2ProcessingStatus, Stage2SpatialTransform, ThresholdMetadata,
        },
        stage3_grading::{
            CORN_THRESHOLDS, FOLD_CHECKPOINT_SHA256, GRADABILITY_THRESHOLD, grade_single_fold,
        },
    };
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    const SHA: &str = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
    const SIDE: u32 = 64;

    #[derive(Clone, Copy)]
    enum Corruption {
        Image,
        Exam,
        Eye,
        Size,
    }

    #[derive(Clone)]
    struct Fake {
        gate_logit: f32,
        grading_logits: [f32; 4],
        fail: HashMap<RequestId, (AnalysisStage, StageFailureKind)>,
        corrupt: Option<(Stage2ModelKind, Corruption)>,
        delay: Duration,
        calls: Arc<Mutex<Vec<(RequestId, AnalysisStage)>>>,
        cancel_when_segmenting: Option<(Stage2ModelKind, Arc<AtomicBool>)>,
    }

    impl Fake {
        fn new(gate_logit: f32, grading_logits: [f32; 4]) -> Self {
            Self {
                gate_logit,
                grading_logits,
                fail: HashMap::new(),
                corrupt: None,
                delay: Duration::ZERO,
                calls: Arc::default(),
                cancel_when_segmenting: None,
            }
        }

        fn note(
            &self,
            request: &AnalysisRequest,
            stage: AnalysisStage,
        ) -> Result<(), StageFailureKind> {
            self.calls.lock().unwrap().push((request.request_id, stage));
            std::thread::sleep(self.delay);
            match self.fail.get(&request.request_id) {
                Some((failing, kind)) if *failing == stage => Err(*kind),
                _ => Ok(()),
            }
        }

        fn stages(&self) -> Vec<AnalysisStage> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|(_, stage)| *stage)
                .collect()
        }
    }

    impl CascadeRunner for Fake {
        fn screening(
            &mut self,
            request: &AnalysisRequest,
        ) -> Result<Stage1GateOutput, StageFailureKind> {
            self.note(request, AnalysisStage::Screening)?;
            Ok(Stage1GateOutput {
                model_id: "fake-stage1".into(),
                version: "1".into(),
                sha256: "b".repeat(64),
                provider: ActualProvider::Cpu,
                preprocessing_version: "fake".into(),
                image: Some(Stage1ImageMetadata {
                    sha256: SHA.into(),
                    width: SIDE,
                    height: SIDE,
                }),
                preprocessing_microseconds: Some(1),
                elapsed_microseconds: 2,
                gate: decode_stage1_gate(&[self.gate_logit]).unwrap(),
            })
        }

        fn segment(
            &mut self,
            request: &AnalysisRequest,
            kind: Stage2ModelKind,
        ) -> Result<Stage2MaskResult, StageFailureKind> {
            self.note(request, AnalysisStage::Segmentation(kind))?;
            if let Some((when, flag)) = &self.cancel_when_segmenting
                && *when == kind
            {
                flag.store(true, Ordering::SeqCst);
            }
            let corruption = self
                .corrupt
                .filter(|(target, _)| *target == kind)
                .map(|(_, c)| c);
            let side = if matches!(corruption, Some(Corruption::Size)) {
                SIDE / 2
            } else {
                SIDE
            };
            let crop = ImageRect {
                x: 0,
                y: 0,
                width: side,
                height: side,
            };
            let transform = match kind {
                Stage2ModelKind::Ma | Stage2ModelKind::He => {
                    Stage2SpatialTransform::LesionCircleStretch {
                        crop,
                        model_size: 1024,
                    }
                }
                Stage2ModelKind::Ex | Stage2ModelKind::Cws => {
                    Stage2SpatialTransform::LesionRectLetterbox {
                        crop,
                        model_size: 1024,
                        resized_width: 1024,
                        resized_height: 1024,
                        offset_x: 0,
                        offset_y: 0,
                    }
                }
                Stage2ModelKind::Vessel => Stage2SpatialTransform::VesselFovSquare {
                    crop,
                    model_size: 512,
                    fov_policy: "hierarchiretina-v1.0-vessel-fov".into(),
                },
            };
            let mut pixels = vec![0u8; (side * side) as usize];
            pixels[5] = 255;
            Ok(Stage2MaskResult {
                processing_status: Stage2ProcessingStatus::Complete,
                error: None,
                examination_id: if matches!(corruption, Some(Corruption::Exam)) {
                    request.examination_id + 1
                } else {
                    request.examination_id
                },
                source_image_sha256: if matches!(corruption, Some(Corruption::Image)) {
                    "c".repeat(64)
                } else {
                    SHA.into()
                },
                laterality: if matches!(corruption, Some(Corruption::Eye)) {
                    Some(EyeLaterality::Left)
                } else {
                    request.eye
                },
                model_kind: kind,
                model_version: "1.0.0".into(),
                checkpoint_sha256: "d".repeat(64),
                model_sha256: "e".repeat(64),
                original_width: side,
                original_height: side,
                mask_width: side,
                mask_height: side,
                transform,
                preprocessing_version: "fake".into(),
                threshold: ThresholdMetadata {
                    value: kind.threshold(),
                    operator: kind.threshold_operator(),
                },
                provider: ActualProvider::Cpu,
                elapsed_microseconds: 3,
                data: MaskPixelData::Binary(pixels),
            })
        }

        fn grade(
            &mut self,
            request: &AnalysisRequest,
            masks: &[Stage2MaskResult],
        ) -> Result<Stage3GradeResult, StageFailureKind> {
            assert_eq!(masks.len(), 5, "grading must receive all five real masks");
            self.note(request, AnalysisStage::Grading)?;
            let decoded = grade_single_fold(&self.grading_logits)
                .map_err(|_| StageFailureKind::InvalidOutput)?;
            Ok(Stage3GradeResult {
                examination_id: request.examination_id,
                source_image_sha256: SHA.into(),
                scope: Stage3Scope::SingleFoldTechnicalTest { fold: 0 },
                model_version: "1.0.0".into(),
                checkpoint_sha256: vec![FOLD_CHECKPOINT_SHA256[0].into()],
                model_sha256: vec!["f".repeat(64)],
                severity_logits: Some(decoded.severity_logits),
                gradability_logit: Some(decoded.gradability_logit),
                full_probabilities: decoded.full,
                ordinal_probabilities: [
                    decoded.probabilities[0],
                    decoded.probabilities[1],
                    decoded.probabilities[2],
                ],
                gradable_probability: decoded.probabilities[3],
                corn_thresholds: CORN_THRESHOLDS,
                gradability_threshold: GRADABILITY_THRESHOLD,
                grade: decoded.grade,
                ungradable: decoded.grade == 5,
                provider: ActualProvider::Cpu,
                elapsed_microseconds: 4,
                research_only: true,
            })
        }
    }

    fn request(id: RequestId) -> AnalysisRequest {
        AnalysisRequest {
            request_id: id,
            examination_id: 40 + id as i64,
            patient_id: Some(9),
            eye: Some(EyeLaterality::Right),
            image_path: PathBuf::from("not-read-by-the-fake.png"),
            policy: AnalysisPolicy::TechnicalFold0,
        }
    }

    fn paths() -> CascadeModelPaths {
        let p = PathBuf::from("unused.json");
        CascadeModelPaths {
            stage1: p.clone(),
            ma: p.clone(),
            he: p.clone(),
            ex: p.clone(),
            cws: p.clone(),
            vessel: p.clone(),
            stage3_fold0: p,
        }
    }

    fn run(
        fake: &mut Fake,
        req: &AnalysisRequest,
        cancelled: &dyn Fn() -> bool,
    ) -> (AnalysisReport, Vec<AnalysisEvent>) {
        let mut events = Vec::new();
        let report = run_cascade(fake, req, cancelled, &mut |e| events.push(e));
        assert_well_formed(&events, req.request_id);
        (report, events)
    }

    /// T14: Started first, exactly one Finished and it is last, completions follow their starts.
    fn assert_well_formed(events: &[AnalysisEvent], id: RequestId) {
        assert!(
            matches!(events.first(), Some(AnalysisEvent::Started { request_id }) if *request_id == id)
        );
        let finished = events
            .iter()
            .filter(|e| matches!(e, AnalysisEvent::Finished(_)))
            .count();
        assert_eq!(finished, 1);
        assert!(matches!(events.last(), Some(AnalysisEvent::Finished(_))));
        let mut open: Option<AnalysisStage> = None;
        for event in events {
            match event {
                AnalysisEvent::StageStarted { stage, .. } => {
                    assert!(open.is_none());
                    open = Some(*stage);
                }
                AnalysisEvent::StageCompleted { stage, .. } => {
                    assert_eq!(open.take(), Some(*stage));
                }
                _ => {}
            }
        }
    }

    const NEVER: &dyn Fn() -> bool = &|| false;

    #[test]
    fn t01_negative_route_skips_every_later_stage_and_reports_stage1_grade0() {
        let mut fake = Fake::new(-6.0, [5.0, 5.0, -5.0, 5.0]);
        let (report, events) = run(&mut fake, &request(1), NEVER);
        assert_eq!(fake.stages(), vec![AnalysisStage::Screening]);
        assert_eq!(report.state, AnalysisState::NegativeRoute);
        assert_eq!(
            report.cascade_grade,
            Some(CascadeGrade::Grade0FromStage1Gate)
        );
        assert!(report.grading.is_none() && report.masks.is_empty());
        let skipped = events
            .iter()
            .filter(|e| matches!(e, AnalysisEvent::StageSkipped { .. }))
            .count();
        assert_eq!(skipped, 6);
    }

    #[test]
    fn t02_referral_runs_five_masks_in_order_then_exactly_one_fold0_grade() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        let (report, events) = run(&mut fake, &request(2), NEVER);
        let expected: Vec<AnalysisStage> = std::iter::once(AnalysisStage::Screening)
            .chain(
                STAGE3_MASK_ORDER
                    .iter()
                    .map(|k| AnalysisStage::Segmentation(*k)),
            )
            .chain(std::iter::once(AnalysisStage::Grading))
            .collect();
        assert_eq!(fake.stages(), expected);
        assert_eq!(report.state, AnalysisState::Graded);
        assert_eq!(report.cascade_grade, Some(CascadeGrade::Severity(3)));
        assert_eq!(report.masks.len(), 5);
        assert!(
            report
                .masks
                .iter()
                .map(|m| m.model_kind)
                .eq(STAGE3_MASK_ORDER)
        );
        let grading = report.grading.as_ref().unwrap();
        assert_eq!(
            grading.scope,
            Stage3Scope::SingleFoldTechnicalTest { fold: 0 }
        );
        assert!(grading.research_only);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, AnalysisEvent::StageSkipped { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn t03_a_failed_screening_is_an_error_never_grade0() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.fail.insert(
            3,
            (AnalysisStage::Screening, StageFailureKind::InvalidOutput),
        );
        let (report, _) = run(&mut fake, &request(3), NEVER);
        assert_eq!(report.state, AnalysisState::Failed);
        assert!(report.cascade_grade.is_none() && report.grading.is_none());
        assert_eq!(
            report.failure.unwrap().kind,
            StageFailureKind::InvalidOutput
        );
        assert_eq!(fake.stages(), vec![AnalysisStage::Screening]);
    }

    #[test]
    fn every_stage1_failure_kind_ends_failed_without_grade0_or_later_stages() {
        for kind in [
            StageFailureKind::InvalidRequest,
            StageFailureKind::InvalidSourceImage,
            StageFailureKind::ModelUnavailable,
            StageFailureKind::ModelIntegrity,
            StageFailureKind::ModelMismatch,
            StageFailureKind::IdentityMismatch,
            StageFailureKind::Inference,
            StageFailureKind::InvalidOutput,
            StageFailureKind::Internal,
        ] {
            // A strongly negative logit would route to grade 0 if the error were swallowed.
            let mut fake = Fake::new(-9.0, [5.0, 5.0, -5.0, 5.0]);
            fake.fail.insert(3, (AnalysisStage::Screening, kind));
            let (report, _) = run(&mut fake, &request(3), NEVER);
            assert_eq!(report.state, AnalysisState::Failed, "{kind:?}");
            assert!(report.cascade_grade.is_none(), "{kind:?}");
            assert!(report.screening.is_none(), "{kind:?}");
            assert_eq!(report.failure.unwrap().kind, kind);
            assert_eq!(fake.stages(), vec![AnalysisStage::Screening], "{kind:?}");
        }
    }

    #[test]
    fn stage1_score_at_threshold_continues_and_just_below_stops_with_no_later_stage() {
        use retinacare_ai_bridge::stage1::STAGE1_GATE_THRESHOLD;
        let logit_at = (STAGE1_GATE_THRESHOLD / (1.0 - STAGE1_GATE_THRESHOLD)).ln();
        for (logit, state, later_stages) in [
            (logit_at - 1e-3, AnalysisState::NegativeRoute, 0),
            (logit_at + 1e-3, AnalysisState::Graded, 6),
        ] {
            let mut fake = Fake::new(logit, [5.0, 5.0, -5.0, 5.0]);
            let (report, _) = run(&mut fake, &request(1), NEVER);
            assert_eq!(report.state, state);
            assert_eq!(fake.stages().len(), 1 + later_stages);
        }
    }

    #[test]
    fn t04_ungradable_reports_no_severity() {
        let mut fake = Fake::new(4.0, [9.0, 9.0, 9.0, -9.0]);
        let (report, _) = run(&mut fake, &request(4), NEVER);
        assert_eq!(report.state, AnalysisState::Ungradable);
        assert_eq!(report.cascade_grade, Some(CascadeGrade::Ungradable));
        assert!(report.grading.as_ref().unwrap().ungradable);
    }

    #[test]
    fn t05_t07_t09_identity_mismatches_fail_before_grading_and_keep_diagnostic_evidence() {
        for (kind, corruption) in [
            (Stage2ModelKind::Ex, Corruption::Image),
            (Stage2ModelKind::He, Corruption::Eye),
            (Stage2ModelKind::Cws, Corruption::Exam),
            (Stage2ModelKind::Ma, Corruption::Size),
        ] {
            let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
            fake.corrupt = Some((kind, corruption));
            let (report, _) = run(&mut fake, &request(5), NEVER);
            assert_eq!(report.state, AnalysisState::Failed, "{kind:?}");
            let failure = report.failure.unwrap();
            assert_eq!(failure.stage, AnalysisStage::Segmentation(kind));
            assert_eq!(failure.kind, StageFailureKind::IdentityMismatch);
            assert!(report.grading.is_none() && report.cascade_grade.is_none());
            assert!(
                !fake.stages().contains(&AnalysisStage::Grading),
                "grading must not run"
            );
            assert!(report.evidence_is_diagnostic_only);
        }
    }

    #[test]
    fn s06_s07_missing_stage_models_fail_typed_without_zero_filled_masks() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.fail.insert(
            6,
            (
                AnalysisStage::Segmentation(Stage2ModelKind::Ex),
                StageFailureKind::ModelUnavailable,
            ),
        );
        let (report, _) = run(&mut fake, &request(6), NEVER);
        assert_eq!(report.state, AnalysisState::Failed);
        assert_eq!(
            report.masks.len(),
            2,
            "only MA and HE completed; no placeholder EX mask"
        );
        assert!(
            !fake
                .stages()
                .contains(&AnalysisStage::Segmentation(Stage2ModelKind::Cws))
        );

        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.fail.insert(
            7,
            (AnalysisStage::Grading, StageFailureKind::ModelUnavailable),
        );
        let (report, _) = run(&mut fake, &request(7), NEVER);
        assert_eq!(report.state, AnalysisState::Failed);
        assert_eq!(report.masks.len(), 5);
        assert!(report.grading.is_none() && report.cascade_grade.is_none());
        assert!(report.evidence_is_diagnostic_only);
    }

    #[test]
    fn t15_t16_cancellation_stops_at_the_next_stage_boundary() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        let (report, _) = run(&mut fake, &request(8), &|| true);
        assert_eq!(report.state, AnalysisState::Cancelled);
        assert_eq!(report.cancelled_before, Some(AnalysisStage::Screening));
        assert!(fake.stages().is_empty(), "no inference once cancelled");

        let flag = Arc::new(AtomicBool::new(false));
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.cancel_when_segmenting = Some((Stage2ModelKind::Ma, flag.clone()));
        let observed = flag.clone();
        let (report, _) = run(&mut fake, &request(9), &move || {
            observed.load(Ordering::SeqCst)
        });
        assert_eq!(report.state, AnalysisState::Cancelled);
        assert_eq!(
            report.cancelled_before,
            Some(AnalysisStage::Segmentation(Stage2ModelKind::He))
        );
        assert_eq!(
            report.masks.len(),
            1,
            "the running MA call finished; nothing started after"
        );
        assert!(report.grading.is_none() && report.cascade_grade.is_none());
    }

    #[test]
    fn t25_every_terminal_variant_serializes_with_research_labels() {
        let scenarios: Vec<AnalysisReport> = vec![
            run(&mut Fake::new(-6.0, [0.0; 4]), &request(10), NEVER).0,
            run(
                &mut Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]),
                &request(11),
                NEVER,
            )
            .0,
            run(
                &mut Fake::new(4.0, [9.0, 9.0, 9.0, -9.0]),
                &request(12),
                NEVER,
            )
            .0,
            run(&mut Fake::new(4.0, [0.0; 4]), &request(13), &|| true).0,
            {
                let mut fake = Fake::new(4.0, [0.0; 4]);
                fake.fail
                    .insert(14, (AnalysisStage::Screening, StageFailureKind::Inference));
                run(&mut fake, &request(14), NEVER).0
            },
        ];
        let states: Vec<_> = scenarios.iter().map(|r| r.state).collect();
        assert_eq!(
            states,
            [
                AnalysisState::NegativeRoute,
                AnalysisState::Graded,
                AnalysisState::Ungradable,
                AnalysisState::Cancelled,
                AnalysisState::Failed
            ]
        );
        for report in scenarios {
            let json = serde_json::to_value(&report).unwrap();
            assert_eq!(json["research_only"], true);
            assert_eq!(json["validation_status"], "technical_only");
            assert_eq!(json["origin"], "official-model-inference");
            assert_eq!(json["policy"], "technical_fold0");
            if let Some(grading) = json.get("grading").filter(|g| !g.is_null()) {
                assert_eq!(grading["scope"]["scope"], "single_fold_technical_test");
                assert_eq!(grading["research_only"], true);
            }
            let text = json.to_string();
            assert!(
                !text.contains("not-read-by-the-fake"),
                "no image path in a report"
            );
        }
    }

    fn collect_until_finished(
        service: &ModelService,
        finished_needed: usize,
    ) -> Vec<AnalysisEvent> {
        let mut events = Vec::new();
        let mut finished = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while finished < finished_needed {
            match service.try_next_analysis() {
                Ok(event) => {
                    if matches!(event, AnalysisEvent::Finished(_)) {
                        finished += 1;
                    }
                    events.push(event);
                }
                Err(_) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "timed out waiting for analysis events"
                    );
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        events
    }

    fn id_of(event: &AnalysisEvent) -> RequestId {
        match event {
            AnalysisEvent::Started { request_id }
            | AnalysisEvent::StageStarted { request_id, .. }
            | AnalysisEvent::StageCompleted { request_id, .. }
            | AnalysisEvent::StageSkipped { request_id, .. } => *request_id,
            AnalysisEvent::Finished(report) => report.request_id,
        }
    }

    #[test]
    fn service_runs_requests_sequentially_without_mixing_examinations() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.delay = Duration::from_millis(2);
        let service = ModelService::with_runner(Box::new(fake));
        service.analyze(request(21), paths()).unwrap();
        service.analyze(request(22), paths()).unwrap();
        let events = collect_until_finished(&service, 2);
        let ids: Vec<_> = events.iter().map(id_of).collect();
        let first_b = ids.iter().position(|id| *id == 22).unwrap();
        assert!(
            ids[..first_b].iter().all(|id| *id == 21),
            "request 21 finishes before 22 starts"
        );
        for event in &events {
            if let AnalysisEvent::Finished(report) = event {
                assert_eq!(report.examination_id, 40 + report.request_id as i64);
                assert_eq!(
                    report
                        .masks
                        .iter()
                        .filter(|m| m.examination_id != report.examination_id)
                        .count(),
                    0
                );
            }
        }
    }

    #[test]
    fn service_backpressure_duplicates_and_queued_cancellation_are_deterministic() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.delay = Duration::from_millis(60);
        let calls = fake.calls.clone();
        let service = ModelService::with_runner(Box::new(fake));
        service.analyze(request(31), paths()).unwrap();
        // Wait until request 31 is running, so the two queue slots are free.
        let mut seen = Vec::new();
        while !matches!(seen.last(), Some(AnalysisEvent::Started { .. })) {
            if let Ok(event) = service.try_next_analysis() {
                seen.push(event);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            service.analyze(request(31), paths()),
            Err(AnalysisSubmitError::DuplicateRequest)
        );
        service.analyze(request(32), paths()).unwrap();
        service.analyze(request(33), paths()).unwrap();
        assert_eq!(
            service.analyze(request(34), paths()),
            Err(AnalysisSubmitError::QueueFull)
        );
        assert!(
            service.cancel_analysis(32),
            "queued request can be cancelled"
        );
        assert!(!service.cancel_analysis(999), "unknown request");
        let mut events = seen;
        events.extend(collect_until_finished(&service, 3));
        let report = |id: RequestId| {
            events.iter().find_map(|e| match e {
                AnalysisEvent::Finished(r) if r.request_id == id => Some(r.clone()),
                _ => None,
            })
        };
        assert_eq!(report(31).unwrap().state, AnalysisState::Graded);
        assert_eq!(report(32).unwrap().state, AnalysisState::Cancelled);
        assert_eq!(report(33).unwrap().state, AnalysisState::Graded);
        assert!(
            calls.lock().unwrap().iter().all(|(id, _)| *id != 32),
            "a request cancelled while queued never reaches a model"
        );
        // A finished request id can be submitted again; the worker is not stuck.
        service.analyze(request(31), paths()).unwrap();
        assert!(collect_until_finished(&service, 1).len() > 1);
    }

    #[test]
    fn t13_real_runner_reports_missing_models_as_typed_failures_and_the_worker_survives() {
        let scratch =
            std::env::temp_dir().join(format!("retinacare-cascade-missing-{}", std::process::id()));
        let missing = scratch.join("manifest.json");
        let all_missing = CascadeModelPaths {
            stage1: missing.clone(),
            ma: missing.clone(),
            he: missing.clone(),
            ex: missing.clone(),
            cws: missing.clone(),
            vessel: missing.clone(),
            stage3_fold0: missing,
        };
        let service = ModelService::new();
        for id in [51, 52] {
            service.analyze(request(id), all_missing.clone()).unwrap();
            let events = collect_until_finished(&service, 1);
            let Some(AnalysisEvent::Finished(report)) = events.last() else {
                panic!("no terminal event")
            };
            assert_eq!(report.state, AnalysisState::Failed);
            let failure = report.failure.as_ref().unwrap();
            assert_eq!(failure.stage, AnalysisStage::Screening);
            assert_eq!(failure.kind, StageFailureKind::ModelUnavailable);
            assert!(
                report.cascade_grade.is_none()
                    && report.grading.is_none()
                    && report.masks.is_empty()
            );
            assert!(
                !serde_json::to_string(report)
                    .unwrap()
                    .contains("retinacare-cascade-missing"),
                "no path in report"
            );
        }
    }

    #[test]
    fn t20_a_failed_request_does_not_block_the_next_good_one() {
        let mut fake = Fake::new(4.0, [5.0, 5.0, -5.0, 5.0]);
        fake.fail.insert(
            41,
            (AnalysisStage::Screening, StageFailureKind::ModelUnavailable),
        );
        let service = ModelService::with_runner(Box::new(fake));
        service.analyze(request(41), paths()).unwrap();
        let first = collect_until_finished(&service, 1);
        assert!(
            matches!(first.last(), Some(AnalysisEvent::Finished(r)) if r.state == AnalysisState::Failed)
        );
        service.analyze(request(42), paths()).unwrap();
        let second = collect_until_finished(&service, 1);
        assert!(
            matches!(second.last(), Some(AnalysisEvent::Finished(r)) if r.state == AnalysisState::Graded)
        );
    }

    /// Real ONNX cascade through the nonblocking `ModelService` on a local technical image.
    /// Env: RETINACARE_CASCADE_{STAGE1,MA,HE,EX,CWS,VESSEL,FOLD0}_MANIFEST, RETINACARE_CASCADE_IMAGE,
    /// RETINACARE_CASCADE_PARITY_ROOT (dirs ma/he/ex/cws/vessel with `<name>_pytorch_mask.u8`).
    #[test]
    #[ignore = "local-only: set RETINACARE_CASCADE_* manifest, image and parity environment variables"]
    fn local_full_cascade_technical_trace_matches_references() {
        let env = |name: &str| {
            PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
        };
        let model_paths = CascadeModelPaths {
            stage1: env("RETINACARE_CASCADE_STAGE1_MANIFEST"),
            ma: env("RETINACARE_CASCADE_MA_MANIFEST"),
            he: env("RETINACARE_CASCADE_HE_MANIFEST"),
            ex: env("RETINACARE_CASCADE_EX_MANIFEST"),
            cws: env("RETINACARE_CASCADE_CWS_MANIFEST"),
            vessel: env("RETINACARE_CASCADE_VESSEL_MANIFEST"),
            stage3_fold0: env("RETINACARE_CASCADE_FOLD0_MANIFEST"),
        };
        let parity_root = env("RETINACARE_CASCADE_PARITY_ROOT");
        let mut request = request(101);
        request.image_path = env("RETINACARE_CASCADE_IMAGE");
        let service = ModelService::new();
        service.analyze(request, model_paths).unwrap();
        let started = std::time::Instant::now();
        let mut trace = Vec::new();
        let report = loop {
            match service.try_next_analysis() {
                Ok(AnalysisEvent::Finished(report)) => {
                    trace.push(format!("Finished({:?})", report.state));
                    break report;
                }
                Ok(event) => trace.push(match event {
                    AnalysisEvent::Started { .. } => "RequestAccepted/Started".to_owned(),
                    AnalysisEvent::StageStarted { stage, .. } => format!("{stage:?} started"),
                    AnalysisEvent::StageCompleted { stage, .. } => format!("{stage:?} completed"),
                    AnalysisEvent::StageSkipped { stage, reason, .. } => {
                        format!("{stage:?} skipped ({reason:?})")
                    }
                    AnalysisEvent::Finished(_) => unreachable!(),
                }),
                Err(_) => {
                    assert!(
                        started.elapsed() < Duration::from_secs(900),
                        "cascade timed out"
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        };
        eprintln!("TRACE: {}", trace.join(" -> "));
        let mut sanitized = report.clone();
        sanitized.masks.clear();
        eprintln!("REPORT: {}", serde_json::to_string(&sanitized).unwrap());
        eprintln!(
            "TIMINGS_US: {:?} total={} peak_rss_kib={:?}",
            report
                .timings
                .iter()
                .map(|t| (t.stage, t.wall_microseconds))
                .collect::<Vec<_>>(),
            report.total_microseconds,
            report.peak_rss_kib
        );
        let screening = report.screening.as_ref().expect("screening ran");
        match report.state {
            AnalysisState::NegativeRoute => {
                assert_eq!(screening.gate.decision, Stage1GateDecision::NegativeRoute);
                assert!(report.masks.is_empty() && report.grading.is_none());
                assert_eq!(
                    report.cascade_grade,
                    Some(CascadeGrade::Grade0FromStage1Gate)
                );
            }
            AnalysisState::Graded | AnalysisState::Ungradable => {
                assert_eq!(screening.gate.decision, Stage1GateDecision::ContinueCascade);
                assert_eq!(report.masks.len(), 5);
                for (mask, name) in report.masks.iter().zip(["ma", "he", "ex", "cws", "vessel"]) {
                    let reference = std::fs::read(
                        parity_root
                            .join(name)
                            .join(format!("{name}_pytorch_mask.u8")),
                    )
                    .unwrap();
                    let MaskPixelData::Binary(pixels) = &mask.data else {
                        panic!("binary mask")
                    };
                    assert_eq!(
                        pixels, &reference,
                        "{name} mask differs from the upstream reference"
                    );
                }
                let grading = report.grading.as_ref().expect("fold 0 ran");
                assert_eq!(
                    grading.scope,
                    Stage3Scope::SingleFoldTechnicalTest { fold: 0 }
                );
                assert!(grading.research_only);
            }
            other => panic!("unexpected terminal state {other:?}: {:?}", report.failure),
        }
    }
}
