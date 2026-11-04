use crate::data::PerformanceData;
use crate::types::{Citation, ProvenanceBlock};

pub fn provenance_for(data: &PerformanceData, citation: &Citation) -> ProvenanceBlock {
    ProvenanceBlock {
        afm_document_number: data.meta.afm_document_number.clone(),
        afm_revision: data.meta.afm_revision.clone(),
        afm_date: data.meta.afm_date.clone(),
        data_status: data.meta.data_status.clone(),
        citation: citation.clone(),
    }
}
