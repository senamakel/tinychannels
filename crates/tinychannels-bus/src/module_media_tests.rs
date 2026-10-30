use super::*;

#[test]
fn media_read_and_upload_requests_round_trip_scope_and_sequence() {
    let request = MediaUploadChunk {
        handle: MediaUploadHandle("handle-1".into()),
        sequence: 2,
        bytes: vec![1, 2, 3],
    };
    let value = serde_json::to_value(&request).unwrap();
    let restored: MediaUploadChunk = serde_json::from_value(value).unwrap();
    assert_eq!(restored.handle.0, "handle-1");
    assert_eq!(restored.sequence, 2);
    assert_eq!(restored.bytes, [1, 2, 3]);
}
