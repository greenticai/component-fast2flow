//! Index building and management logic.

use std::collections::BTreeMap;

use chrono::Utc;
use greentic_types::cbor::canonical;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A single flow entry in the index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowEntry {
    pub pack_id: String,
    pub flow_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

/// The complete flow index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowIndex {
    pub version: String,
    pub last_updated: String,
    pub flows: Vec<FlowEntry>,
    /// Pre-computed term frequencies for BM25.
    #[serde(default)]
    pub term_frequencies: BTreeMap<String, BTreeMap<String, u32>>,
    /// Document frequencies for terms.
    #[serde(default)]
    pub document_frequencies: BTreeMap<String, u32>,
}

/// Input for build operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildInput {
    pub flows: Vec<FlowEntry>,
    pub tenant_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
}

/// Input for update operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateInput {
    pub flows: Vec<FlowEntry>,
    pub tenant_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub mode: String, // "add", "remove", "replace"
}

/// Output for build/update operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexOutput {
    pub version: String,
    pub last_updated: String,
    pub flow_count: usize,
    pub index_key: String,
    #[serde(default)]
    pub index: Option<FlowIndex>,
}

/// Build a new index from flow metadata.
pub fn build_index(input: Vec<u8>) -> Vec<u8> {
    let result = do_build_index(&input);
    canonical::to_canonical_cbor_allow_floats(&result).unwrap_or_default()
}

/// Update an existing index.
pub fn update_index(input: Vec<u8>) -> Vec<u8> {
    let result = do_update_index(&input);
    canonical::to_canonical_cbor_allow_floats(&result).unwrap_or_default()
}

fn do_build_index(input: &[u8]) -> JsonValue {
    let input_value: JsonValue = match canonical::from_cbor(input) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("failed to parse input: {}", e)
            });
        }
    };

    let build_input: BuildInput = match serde_json::from_value(input_value) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("invalid input structure: {}", e)
            });
        }
    };

    let index = create_index(build_input.flows);
    let index_key = format!(
        "fast2flow:index:{}:{}",
        build_input.tenant_id,
        build_input.team_id.as_deref().unwrap_or("default")
    );

    let output = IndexOutput {
        version: index.version.clone(),
        last_updated: index.last_updated.clone(),
        flow_count: index.flows.len(),
        index_key,
        index: Some(index),
    };

    serde_json::to_value(output).unwrap_or_else(|_| serde_json::json!({}))
}

fn do_update_index(input: &[u8]) -> JsonValue {
    let input_value: JsonValue = match canonical::from_cbor(input) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("failed to parse input: {}", e)
            });
        }
    };

    let update_input: UpdateInput = match serde_json::from_value(input_value) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("invalid input structure: {}", e)
            });
        }
    };

    // For simplicity, just rebuild the index with the new flows
    // In a real implementation, this would merge with existing index
    let index = create_index(update_input.flows);
    let index_key = format!(
        "fast2flow:index:{}:{}",
        update_input.tenant_id,
        update_input.team_id.as_deref().unwrap_or("default")
    );

    let output = IndexOutput {
        version: index.version.clone(),
        last_updated: index.last_updated.clone(),
        flow_count: index.flows.len(),
        index_key,
        index: Some(index),
    };

    serde_json::to_value(output).unwrap_or_else(|_| serde_json::json!({}))
}

/// Create a new index with pre-computed term frequencies.
fn create_index(flows: Vec<FlowEntry>) -> FlowIndex {
    let mut term_frequencies: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
    let mut document_frequencies: BTreeMap<String, u32> = BTreeMap::new();

    for flow in &flows {
        let doc_id = format!("{}:{}", flow.pack_id, flow.flow_id);
        let mut doc_terms: BTreeMap<String, u32> = BTreeMap::new();

        // Tokenize and count terms from various fields
        let all_text = format!(
            "{} {} {} {}",
            flow.title,
            flow.description.as_deref().unwrap_or(""),
            flow.tags.join(" "),
            flow.keywords.join(" ")
        );

        for term in tokenize(&all_text) {
            *doc_terms.entry(term.clone()).or_insert(0) += 1;
        }

        // Update document frequencies
        for term in doc_terms.keys() {
            *document_frequencies.entry(term.clone()).or_insert(0) += 1;
        }

        term_frequencies.insert(doc_id, doc_terms);
    }

    FlowIndex {
        version: "1.0".to_string(),
        last_updated: Utc::now().to_rfc3339(),
        flows,
        term_frequencies,
        document_frequencies,
    }
}

/// Simple tokenizer that splits on whitespace and punctuation.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Book an Appointment!");
        assert!(tokens.contains(&"book".to_string()));
        assert!(tokens.contains(&"an".to_string()));
        assert!(tokens.contains(&"appointment".to_string()));
    }

    #[test]
    fn test_create_index() {
        let flows = vec![FlowEntry {
            pack_id: "test-pack".to_string(),
            flow_id: "test-flow".to_string(),
            title: "Book Appointment".to_string(),
            description: Some("Schedule meetings".to_string()),
            tags: vec!["booking".to_string()],
            keywords: vec!["schedule".to_string()],
        }];

        let index = create_index(flows);
        assert_eq!(index.flows.len(), 1);
        assert!(!index.term_frequencies.is_empty());
        assert!(!index.document_frequencies.is_empty());
    }
}
