use super::*;

#[test]
fn collections_is_a_unique_complete_owner_inventory() {
    let specs = collections();
    let names: Vec<_> = specs.iter().map(|spec| spec.name.as_str()).collect();
    assert_eq!(names, COLLECTIONS);
    assert_eq!(names.len(), 16);
    for name in [MEDIA_METADATA, MIGRATIONS] {
        assert!(names.contains(&name));
    }
    assert!(specs.iter().all(|spec| spec.indexes.is_empty()));
    assert_eq!(media_metadata_spec().name, MEDIA_METADATA);
}
