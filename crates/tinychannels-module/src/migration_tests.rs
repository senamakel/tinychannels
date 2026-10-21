use super::*;
use tinychannels::storage::schema::{COLLECTIONS, OWNER, SCHEMA_VERSION, storage_schema};
use tinystoragedrivers_core::StorageError;
use tinystoragedrivers_core::schema::{
    BlobTargetRequest, Item, Step, StreamEntriesRequest, StreamPlanRequest, TransformKind,
};
use tinystoragedrivers_tinybus::migration::{
    DocumentsTransformRequest, MigrationPayload, MigrationPayloadReply,
};

const EXPECTED_STEPS: [(&str, &str, &str); 14] = [
    (
        "delivery-pending-v2",
        "channels_delivery_pending",
        "channels.delivery_pending",
    ),
    (
        "delivery-failed-v2",
        "channels_delivery_failed",
        "channels.delivery_failed",
    ),
    (
        "delivery-results-v2",
        "channels_delivery_results",
        "channels.delivery_results",
    ),
    (
        "remote-bindings-v2",
        "channel_remote_bindings",
        "channels.remote_bindings",
    ),
    (
        "remote-busy-v2",
        "channel_remote_busy",
        "channels.remote_busy",
    ),
    (
        "remote-meta-v2",
        "channel_remote_meta",
        "channels.remote_meta",
    ),
    (
        "whatsapp-device-v2",
        "whatsapp_device",
        "channels.whatsapp_device",
    ),
    (
        "whatsapp-identity-v2",
        "whatsapp_identity",
        "channels.whatsapp_identity",
    ),
    (
        "whatsapp-session-v2",
        "whatsapp_session",
        "channels.whatsapp_session",
    ),
    (
        "whatsapp-prekey-v2",
        "whatsapp_prekey",
        "channels.whatsapp_prekey",
    ),
    (
        "whatsapp-signed-prekey-v2",
        "whatsapp_signed_prekey",
        "channels.whatsapp_signed_prekey",
    ),
    (
        "whatsapp-sender-key-v2",
        "whatsapp_sender_key",
        "channels.whatsapp_sender_key",
    ),
    (
        "whatsapp-sync-key-v2",
        "whatsapp_sync_key",
        "channels.whatsapp_sync_key",
    ),
    (
        "whatsapp-lid-mapping-v2",
        "whatsapp_lid_mapping",
        "channels.whatsapp_lid_mapping",
    ),
];

#[test]
fn declaration_exports_all_fourteen_module_scoped_document_steps() {
    let schema = storage_schema();
    let declaration = declaration_from_owner_schema(&schema);
    assert!(
        declaration.is_ok(),
        "every declared step has an operation mapping"
    );
    let Some(declaration) = declaration.ok() else {
        return;
    };

    assert_eq!(declaration.owner.as_str(), OWNER);
    assert_eq!(declaration.items.len(), COLLECTIONS.len());
    assert_eq!(declaration.steps.len(), EXPECTED_STEPS.len());
    for (item, expected_name) in declaration.items.iter().zip(COLLECTIONS) {
        assert!(matches!(
            item,
            Item::Collection { name, version, scope, .. }
                if name == expected_name
                    && *version == SCHEMA_VERSION
                    && *scope == ScopeClass::Module
        ));
    }

    for (step, (step_id, source, item)) in declaration.steps.iter().zip(EXPECTED_STEPS) {
        assert_eq!(step.id, step_id);
        assert_eq!(step.item, item);
        assert_eq!(step.source.as_deref(), Some(source));
        assert_eq!(step.from, 1);
        assert_eq!(step.to, 2);
        assert_eq!(step.transform.kind, TransformKind::Documents);
        assert_eq!(
            step.transform.operation,
            operation_selector(step_id).unwrap_or_default()
        );
        assert_eq!(step.transform.version, OPERATION_VERSION);
    }
}

#[test]
fn each_existing_owner_selector_dispatches_to_its_actual_document_callback() {
    let handler = ChannelMigration {
        schema: storage_schema(),
    };
    assert_eq!(handler.schema.steps.len(), EXPECTED_STEPS.len());
    for (step, (step_id, _, _)) in handler.schema.steps.iter().zip(EXPECTED_STEPS) {
        assert_eq!(step.id, step_id);
        let Some(operation) = operation_selector(step.id) else {
            assert_eq!(operation_selector(step.id), Some("missing-selector"));
            continue;
        };
        let reply = handler.transform(MigrationPayload::Documents(DocumentsTransformRequest {
            operation: operation.to_owned(),
            version: OPERATION_VERSION,
            records: Vec::new(),
        }));
        assert!(matches!(
            reply,
            MigrationPayloadReply::Documents(Ok(records)) if records.is_empty()
        ));
    }
}

#[test]
fn unmapped_steps_and_non_module_items_fail_closed() {
    let schema = storage_schema().step(Step::new(
        "unmapped-step",
        "channels.delivery_pending",
        2,
        3,
        |_records| Ok(Vec::new()),
    ));
    assert!(declaration_from_owner_schema(&schema).is_err());

    let non_module =
        storage_schema().collection("channels.untrusted_fixture", 1, ScopeClass::Tenant);
    assert!(validate_module_scope(&non_module).is_err());
}

#[test]
fn unknown_operations_and_versions_are_rejected() {
    let handler = ChannelMigration {
        schema: storage_schema(),
    };
    let reply = handler.transform(MigrationPayload::Documents(DocumentsTransformRequest {
        operation: "unknown.channel.operation".to_owned(),
        version: OPERATION_VERSION,
        records: Vec::new(),
    }));
    assert!(matches!(
        reply,
        MigrationPayloadReply::Documents(Err(TransformError {
            kind: TransformErrorKind::UnsupportedOperation,
            ..
        }))
    ));

    let Some(operation) = operation_selector(EXPECTED_STEPS[0].0) else {
        return;
    };
    let reply = handler.transform(MigrationPayload::Documents(DocumentsTransformRequest {
        operation: operation.to_owned(),
        version: OPERATION_VERSION + 1,
        records: Vec::new(),
    }));
    assert!(matches!(
        reply,
        MigrationPayloadReply::Documents(Err(TransformError {
            kind: TransformErrorKind::UnsupportedVersion,
            ..
        }))
    ));
}

#[test]
fn document_dispatch_rejects_stream_and_blob_steps() {
    let mut stream_schema = storage_schema();
    stream_schema.steps[0].run = StepBody::Streams(Arc::new(|_, _| Ok(None)));
    let mut blob_schema = storage_schema();
    blob_schema.steps[0].run = StepBody::Blobs(Arc::new(|_, _, _| Ok(None)));

    for schema in [stream_schema, blob_schema] {
        let handler = ChannelMigration { schema };
        let reply = handler.transform(MigrationPayload::Documents(DocumentsTransformRequest {
            operation: operation_selector(EXPECTED_STEPS[0].0).unwrap().to_owned(),
            version: OPERATION_VERSION,
            records: Vec::new(),
        }));
        assert!(matches!(
            reply,
            MigrationPayloadReply::Documents(Err(TransformError {
                kind: TransformErrorKind::UnsupportedOperation,
                ..
            }))
        ));
    }
}

#[test]
fn non_document_payloads_are_rejected_with_their_matching_reply_kind() {
    let handler = ChannelMigration {
        schema: storage_schema(),
    };
    let replies = [
        handler.transform(MigrationPayload::BlobTarget(BlobTargetRequest {
            operation: "channels.blob".to_owned(),
            version: OPERATION_VERSION,
            source: "legacy/blob".to_owned(),
        })),
        handler.transform(MigrationPayload::StreamPlan(StreamPlanRequest {
            operation: "channels.stream-plan".to_owned(),
            version: OPERATION_VERSION,
            source: "legacy/stream".to_owned(),
        })),
        handler.transform(MigrationPayload::StreamEntries(StreamEntriesRequest {
            operation: "channels.stream-entries".to_owned(),
            version: OPERATION_VERSION,
            entries: Vec::new(),
        })),
    ];
    assert!(matches!(
        replies[0],
        MigrationPayloadReply::BlobTarget(Err(TransformError {
            kind: TransformErrorKind::UnsupportedOperation,
            ..
        }))
    ));
    assert!(matches!(
        replies[1],
        MigrationPayloadReply::StreamPlan(Err(TransformError {
            kind: TransformErrorKind::UnsupportedOperation,
            ..
        }))
    ));
    assert!(matches!(
        replies[2],
        MigrationPayloadReply::StreamEntries(Err(TransformError {
            kind: TransformErrorKind::UnsupportedOperation,
            ..
        }))
    ));
}

#[test]
fn owner_transform_errors_are_returned_as_sanitized_invalid_input() {
    let mut schema = storage_schema();
    schema.steps[0] = Step::new(
        "delivery-pending-v2",
        "channels.delivery_pending",
        1,
        2,
        |_| Err(StorageError::invalid_input("private backend detail")),
    )
    .source("channels_delivery_pending");
    let handler = ChannelMigration { schema };

    let reply = handler.transform(MigrationPayload::Documents(DocumentsTransformRequest {
        operation: operation_selector(EXPECTED_STEPS[0].0).unwrap().to_owned(),
        version: OPERATION_VERSION,
        records: Vec::new(),
    }));
    assert!(matches!(
        reply,
        MigrationPayloadReply::Documents(Err(TransformError {
            kind: TransformErrorKind::InvalidInput,
            message,
        })) if message == "channel migration transform failed"
    ));
}
