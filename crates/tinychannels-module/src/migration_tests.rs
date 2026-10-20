use super::*;
use tinychannels::storage::schema::{COLLECTIONS, OWNER, SCHEMA_VERSION, storage_schema};
use tinystoragedrivers_core::schema::{Item, Step, TransformKind};
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
