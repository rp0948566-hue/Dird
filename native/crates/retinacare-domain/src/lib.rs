use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Patient {
    pub id: Option<i64>,
    pub patient_code: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Examination {
    pub id: Option<i64>,
    pub patient_id: i64,
    pub examined_at: String,
    pub operator: String,
    pub notes: String,
    #[serde(default)]
    pub eye: String,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub analysis_json: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub class_id: String,
    pub confidence: f32,
    pub bounds_xywh: [f32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RetinalAnnotation {
    pub image_id: i64,
    pub class_id: String,
    pub geometry: AnnotationGeometry,
    pub origin: AnnotationOrigin,
    pub confidence: Option<f32>,
    pub modified_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "coordinates", rename_all = "snake_case")]
pub enum AnnotationGeometry {
    BoundingBox([f32; 4]),
    Polygon(Vec<[f32; 2]>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationOrigin {
    Ai,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClinicalClassification {
    pub guideline_id: String,
    pub guideline_version: String,
    pub severity_id: String,
    pub urgency: Option<String>,
    pub followup_days: Option<u32>,
    pub manually_modified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelAvailability {
    pub model_id: String,
    pub status: String,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patient_roundtrips_through_json() {
        let patient = Patient {
            id: None,
            patient_code: "local-1".into(),
            display_name: "Test Patient".into(),
        };
        let json = serde_json::to_string(&patient).unwrap();
        assert_eq!(serde_json::from_str::<Patient>(&json).unwrap(), patient);
    }

    #[test]
    fn manual_annotation_retains_ai_origin_and_geometry() {
        let annotation = RetinalAnnotation {
            image_id: 8,
            class_id: "microaneurysm".into(),
            geometry: AnnotationGeometry::BoundingBox([1.0, 2.0, 3.0, 4.0]),
            origin: AnnotationOrigin::Ai,
            confidence: Some(0.92),
            modified_at: Some("2026-10-26T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&annotation).unwrap();
        assert_eq!(
            serde_json::from_str::<RetinalAnnotation>(&json).unwrap(),
            annotation
        );
    }
}
