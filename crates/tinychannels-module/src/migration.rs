//! Schema discovery and owner transforms for the compiled TinyChannels module.

use std::sync::Arc;

use tinystoragedrivers_core::schema::{
    Schema, SchemaDeclaration, ScopeClass, Step, StepBody, TransformError, TransformErrorKind,
    declaration_from_schema,
};
use tinystoragedrivers_tinybus::migration::{
    MigrationPayload, MigrationPayloadHandler, MigrationPayloadReply, serve_module_migration,
};

const OPERATION_VERSION: u32 = 1;

fn operation_selector(step_id: &str) -> Option<&'static str> {
    match step_id {
        "delivery-pending-v2" => Some("channels.delivery-pending-v2"),
        "delivery-failed-v2" => Some("channels.delivery-failed-v2"),
        "delivery-results-v2" => Some("channels.delivery-results-v2"),
        "remote-bindings-v2" => Some("channels.remote-bindings-v2"),
        "remote-busy-v2" => Some("channels.remote-busy-v2"),
        "remote-meta-v2" => Some("channels.remote-meta-v2"),
        "whatsapp-device-v2" => Some("channels.whatsapp-device-v2"),
        "whatsapp-identity-v2" => Some("channels.whatsapp-identity-v2"),
        "whatsapp-session-v2" => Some("channels.whatsapp-session-v2"),
        "whatsapp-prekey-v2" => Some("channels.whatsapp-prekey-v2"),
        "whatsapp-signed-prekey-v2" => Some("channels.whatsapp-signed-prekey-v2"),
        "whatsapp-sender-key-v2" => Some("channels.whatsapp-sender-key-v2"),
        "whatsapp-sync-key-v2" => Some("channels.whatsapp-sync-key-v2"),
        "whatsapp-lid-mapping-v2" => Some("channels.whatsapp-lid-mapping-v2"),
        _ => None,
    }
}

fn declaration_from_owner_schema(schema: &Schema) -> tinybus::Result<SchemaDeclaration> {
    declaration_from_schema(schema, |step| {
        operation_selector(step.id).map(|operation| (operation.to_owned(), OPERATION_VERSION))
    })
    .map_err(|_| tinybus::Error::failed("channel migration schema is invalid"))
}

fn validate_module_scope(schema: &Schema) -> tinybus::Result<()> {
    if schema
        .items
        .iter()
        .any(|item| item.scope() != ScopeClass::Module)
    {
        return Err(tinybus::Error::failed(
            "channel migration schema contains a non-module item",
        ));
    }
    Ok(())
}

struct ChannelMigration {
    schema: Schema,
}

impl ChannelMigration {
    fn step(&self, operation: &str, version: u32) -> Result<&Step, TransformError> {
        let Some(step) = self
            .schema
            .steps
            .iter()
            .find(|step| operation_selector(step.id) == Some(operation))
        else {
            return Err(unsupported_operation());
        };
        if version != OPERATION_VERSION {
            return Err(unsupported_version());
        }
        Ok(step)
    }

    fn documents(
        &self,
        operation: &str,
        version: u32,
        records: Vec<tinystoragedrivers_core::schema::SourceRecord>,
    ) -> MigrationPayloadReply {
        let result = self
            .step(operation, version)
            .and_then(|step| match &step.run {
                StepBody::Documents(transform) => {
                    transform(records).map_err(|_| invalid_transform())
                }
                StepBody::Streams(_) | StepBody::Blobs(_) => Err(unsupported_operation()),
            });
        MigrationPayloadReply::Documents(result)
    }
}

impl MigrationPayloadHandler for ChannelMigration {
    fn transform(&self, payload: MigrationPayload) -> MigrationPayloadReply {
        match payload {
            MigrationPayload::Documents(request) => {
                self.documents(&request.operation, request.version, request.records)
            }
            MigrationPayload::BlobTarget(_) => {
                MigrationPayloadReply::BlobTarget(Err(unsupported_operation()))
            }
            MigrationPayload::StreamPlan(_) => {
                MigrationPayloadReply::StreamPlan(Err(unsupported_operation()))
            }
            MigrationPayload::StreamEntries(_) => {
                MigrationPayloadReply::StreamEntries(Err(unsupported_operation()))
            }
        }
    }
}

fn unsupported_operation() -> TransformError {
    TransformError {
        kind: TransformErrorKind::UnsupportedOperation,
        message: "channel migration operation is unsupported".to_owned(),
    }
}

fn unsupported_version() -> TransformError {
    TransformError {
        kind: TransformErrorKind::UnsupportedVersion,
        message: "channel migration operation version is unsupported".to_owned(),
    }
}

fn invalid_transform() -> TransformError {
    TransformError {
        kind: TransformErrorKind::InvalidInput,
        message: "channel migration transform failed".to_owned(),
    }
}

pub(crate) async fn setup(context: &tinybus_module::ModuleSetupContext) -> tinybus::Result<()> {
    let schema = tinychannels::storage::storage_schema();
    validate_module_scope(&schema)?;
    let declaration = declaration_from_owner_schema(&schema)?;
    serve_module_migration(context, declaration, Arc::new(ChannelMigration { schema })).await
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
