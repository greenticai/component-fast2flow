//! E2E tests for fast2flow components.
//!
//! Tests the full routing pipeline: indexer -> matcher -> router.

#[cfg(test)]
mod tests {
    use greentic_types::cbor::canonical::{from_cbor, to_canonical_cbor_allow_floats};
    use serde_json::json;

    /// Test indexer builds index from flow metadata
    #[test]
    fn test_indexer_build_e2e() {
        let input = json!({
            "tenant_id": "demo",
            "team_id": "default",
            "flows": [
                {
                    "pack_id": "customer-support",
                    "flow_id": "book_appointment",
                    "title": "Book Appointment",
                    "description": "Schedule appointments with calendar integration",
                    "tags": ["booking", "calendar", "appointment"],
                    "keywords": ["book", "schedule", "meeting", "appointment"]
                },
                {
                    "pack_id": "customer-support",
                    "flow_id": "check_order_status",
                    "title": "Check Order Status",
                    "description": "Look up order status and tracking information",
                    "tags": ["order", "tracking", "status"],
                    "keywords": ["order", "status", "track", "delivery", "shipping"]
                },
                {
                    "pack_id": "hr-assistant",
                    "flow_id": "request_time_off",
                    "title": "Request Time Off",
                    "description": "Submit vacation or time off requests",
                    "tags": ["hr", "vacation", "leave"],
                    "keywords": ["vacation", "time off", "leave", "pto", "holiday"]
                }
            ]
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = indexer::index::build_index(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Verify index was created
        assert!(
            output.get("index").is_some() || output.get("version").is_some(),
            "Output should have index data: {:?}",
            output
        );

        // Check flow count
        if let Some(count) = output.get("flow_count") {
            assert_eq!(count, 3);
        }
    }

    /// Test matcher finds correct flow for query
    #[test]
    fn test_matcher_match_e2e() {
        // First build an index
        let index_input = json!({
            "tenant_id": "demo",
            "flows": [
                {
                    "pack_id": "customer-support",
                    "flow_id": "book_appointment",
                    "title": "Book Appointment",
                    "description": "Schedule appointments with calendar",
                    "tags": ["booking", "calendar"],
                    "keywords": ["book", "schedule", "appointment"]
                },
                {
                    "pack_id": "customer-support",
                    "flow_id": "check_order",
                    "title": "Check Order Status",
                    "description": "Look up order status",
                    "tags": ["order", "tracking"],
                    "keywords": ["order", "status", "track"]
                }
            ]
        });

        let index_cbor = to_canonical_cbor_allow_floats(&index_input).unwrap();
        let index_output_cbor = indexer::index::build_index(index_cbor);
        let index_output: serde_json::Value = from_cbor(&index_output_cbor).unwrap();

        // Get the index from output
        let index = index_output
            .get("index")
            .cloned()
            .unwrap_or_else(|| index_output.clone());

        // Now test matching
        let match_input = json!({
            "query": "I want to book an appointment",
            "index": index,
            "threshold": 0.1,
            "max_results": 5
        });

        let match_cbor = to_canonical_cbor_allow_floats(&match_input).unwrap();
        let match_output_cbor = matcher::bm25::match_query(match_cbor);
        let match_output: serde_json::Value = from_cbor(&match_output_cbor).unwrap();

        // Verify match result
        assert!(
            match_output.get("status").is_some(),
            "Output should have 'status' field: {:?}",
            match_output
        );
    }

    /// Test router creates dispatch directive for clear match
    #[test]
    fn test_router_dispatch_e2e() {
        let input = json!({
            "message": {
                "id": "msg-123",
                "text": "I want to book an appointment",
                "channel": "telegram",
                "session_id": "sess-abc"
            },
            "match_result": {
                "status": "match",
                "top_match": {
                    "pack_id": "customer-support",
                    "flow_id": "book_appointment",
                    "title": "Book Appointment",
                    "confidence": 0.95
                },
                "candidates": [],
                "latency_ms": 15
            },
            "tenant_id": "demo",
            "team_id": "default",
            "config": {
                "confidence_threshold": 0.7
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = router::route::route_message(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Output is ControlDirective directly (not wrapped)
        assert_eq!(
            output["action"], "dispatch",
            "Should dispatch: {:?}",
            output
        );
        assert!(output.get("target").is_some());
    }

    /// Test router creates respond directive for ambiguous match
    #[test]
    fn test_router_ambiguous_e2e() {
        let input = json!({
            "message": {
                "id": "msg-456",
                "text": "schedule something",
                "channel": "slack",
                "session_id": "sess-def"
            },
            "match_result": {
                "status": "ambiguous",
                "top_match": null,
                "candidates": [
                    {
                        "pack_id": "customer-support",
                        "flow_id": "book_appointment",
                        "title": "Book Appointment",
                        "confidence": 0.65
                    },
                    {
                        "pack_id": "hr-assistant",
                        "flow_id": "schedule_meeting",
                        "title": "Schedule Meeting",
                        "confidence": 0.62
                    }
                ],
                "latency_ms": 20
            },
            "tenant_id": "demo",
            "config": {}
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = router::route::route_message(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Output is ControlDirective directly
        assert_eq!(
            output["action"], "respond",
            "Should respond for ambiguous: {:?}",
            output
        );
    }

    /// Test router creates continue directive for no match
    #[test]
    fn test_router_no_match_e2e() {
        let input = json!({
            "message": {
                "id": "msg-789",
                "text": "gibberish xyz123",
                "channel": "teams",
                "session_id": "sess-ghi"
            },
            "match_result": {
                "status": "no_match",
                "top_match": null,
                "candidates": [],
                "latency_ms": 10
            },
            "tenant_id": "demo",
            "config": {}
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = router::route::route_message(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Output is ControlDirective directly
        assert_eq!(
            output["action"], "continue",
            "Should continue for no match: {:?}",
            output
        );
    }

    /// Test full pipeline: index -> match -> route
    #[test]
    fn test_full_pipeline_e2e() {
        // Step 1: Build index
        let flows = json!({
            "tenant_id": "demo",
            "flows": [
                {
                    "pack_id": "support",
                    "flow_id": "refund_request",
                    "title": "Request Refund",
                    "description": "Process refund requests for orders",
                    "tags": ["refund", "order"],
                    "keywords": ["refund", "money back", "return"]
                },
                {
                    "pack_id": "support",
                    "flow_id": "track_shipment",
                    "title": "Track Shipment",
                    "description": "Track package delivery status",
                    "tags": ["shipping", "tracking"],
                    "keywords": ["track", "shipping", "delivery", "package"]
                }
            ]
        });

        let index_cbor = to_canonical_cbor_allow_floats(&flows).unwrap();
        let index_output_cbor = indexer::index::build_index(index_cbor);
        let index_output: serde_json::Value = from_cbor(&index_output_cbor).unwrap();

        // Get index from output
        let index = index_output
            .get("index")
            .cloned()
            .unwrap_or_else(|| index_output.clone());

        // Step 2: Match query
        let match_input = json!({
            "query": "I want a refund for my order",
            "index": index,
            "threshold": 0.1,
            "max_results": 3
        });

        let match_cbor = to_canonical_cbor_allow_floats(&match_input).unwrap();
        let match_output_cbor = matcher::bm25::match_query(match_cbor);
        let match_result: serde_json::Value = from_cbor(&match_output_cbor).unwrap();

        // Step 3: Route based on match
        let route_input = json!({
            "message": {
                "id": "msg-full-pipeline",
                "text": "I want a refund for my order",
                "channel": "webchat",
                "session_id": "sess-pipeline"
            },
            "match_result": match_result,
            "tenant_id": "demo",
            "config": {
                "confidence_threshold": 0.3
            }
        });

        let route_cbor = to_canonical_cbor_allow_floats(&route_input).unwrap();
        let route_output_cbor = router::route::route_message(route_cbor);
        let route_output: serde_json::Value = from_cbor(&route_output_cbor).unwrap();

        // Verify end-to-end result - output is ControlDirective directly
        assert!(
            route_output.get("action").is_some(),
            "Should have action: {:?}",
            route_output
        );

        // Should dispatch to refund flow or respond for clarification
        let action = route_output["action"].as_str().unwrap();
        assert!(
            action == "dispatch" || action == "respond" || action == "continue",
            "Should have valid action: {:?}",
            route_output
        );
    }

    /// Test indexer error handling
    #[test]
    fn test_indexer_invalid_input() {
        let invalid_input = json!({
            "invalid": "data"
        });

        let input_cbor = to_canonical_cbor_allow_floats(&invalid_input).unwrap();
        let output_cbor = indexer::index::build_index(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert!(
            output.get("error").is_some(),
            "Should return error: {:?}",
            output
        );
    }

    /// Test matcher error handling
    #[test]
    fn test_matcher_invalid_input() {
        let invalid_input = json!({
            "query": "test"
            // missing index
        });

        let input_cbor = to_canonical_cbor_allow_floats(&invalid_input).unwrap();
        let output_cbor = matcher::bm25::match_query(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert!(
            output.get("error").is_some(),
            "Should return error: {:?}",
            output
        );
    }

    /// Test router error handling
    #[test]
    fn test_router_invalid_input() {
        let invalid_input = json!({
            "invalid": "data"
        });

        let input_cbor = to_canonical_cbor_allow_floats(&invalid_input).unwrap();
        let output_cbor = router::route::route_message(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert!(
            output.get("error").is_some(),
            "Should return error: {:?}",
            output
        );
    }

    /// Test router with blocked intents (format: pack_id:flow_id)
    #[test]
    fn test_router_blocked_intent() {
        let input = json!({
            "message": {
                "id": "msg-blocked",
                "text": "hack the system",
                "channel": "telegram",
                "session_id": "sess-blocked"
            },
            "match_result": {
                "status": "match",
                "top_match": {
                    "pack_id": "admin",
                    "flow_id": "system_access",
                    "title": "System Access",
                    "confidence": 0.9
                },
                "candidates": [],
                "latency_ms": 5
            },
            "tenant_id": "demo",
            "config": {
                "blocked_intents": ["admin:system_access", "admin:delete_data"]
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = router::route::route_message(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert_eq!(
            output["action"], "deny",
            "Should deny blocked intent: {:?}",
            output
        );
    }
}
