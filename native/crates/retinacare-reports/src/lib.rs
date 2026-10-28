use retinacare_domain::{Examination, Finding, Patient};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportData {
    pub patient: Patient,
    pub examination: Examination,
    pub findings: Vec<Finding>,
    pub model_id: Option<String>,
    pub clinical_review_required: bool,
}

impl ReportData {
    pub fn from_records(
        patient: Patient,
        examination: Examination,
        findings: Vec<Finding>,
        model_id: Option<String>,
    ) -> Self {
        Self {
            patient,
            examination,
            findings,
            model_id,
            clinical_review_required: true,
        }
    }
}
