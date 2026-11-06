use crate::rev29::types::{Rev29File, SourceMeta};
use crate::types::Citation;

pub fn meta_document_number(data: &Rev29File) -> String {
    data.meta.doc.clone()
}

pub fn meta_revision(data: &Rev29File) -> String {
    data.meta.revision.to_string()
}

pub fn meta_date(data: &Rev29File) -> String {
    data.meta.revision_date.clone()
}

pub fn meta_data_status(data: &Rev29File) -> String {
    data.meta
        .data_status
        .clone()
        .unwrap_or_else(|| "production".into())
}

pub fn afm_official_url(data: &Rev29File) -> String {
    data.meta
        .afm_official_url
        .clone()
        .unwrap_or_else(|| data.meta.source_url.clone())
}

pub fn amm_official_url(data: &Rev29File) -> String {
    data.meta.amm_official_url.clone().unwrap_or_default()
}

pub fn citation_from_source(source: &SourceMeta) -> Citation {
    Citation {
        figure_or_table: source.figure.clone(),
        page: parse_afm_page(&source.afm_page),
        afm_revision: source.page_revision.clone(),
        notes: source.note.clone(),
    }
}

fn parse_afm_page(p: &str) -> Option<u32> {
    p.split('/')
        .next()
        .and_then(|s| s.strip_prefix("5-"))
        .and_then(|s| s.parse().ok())
}

pub fn provenance_for(data: &Rev29File, source: &SourceMeta) -> crate::types::ProvenanceBlock {
    crate::types::ProvenanceBlock {
        afm_document_number: meta_document_number(data),
        afm_revision: meta_revision(data),
        afm_date: meta_date(data),
        data_status: meta_data_status(data),
        citation: citation_from_source(source),
    }
}
