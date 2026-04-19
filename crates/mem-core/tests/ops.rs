use mem_core::types::{ListParams, RelatedParams, SearchParams, ShowParams, UpdateParams};
use mem_core::{note, search, tags, vault};
use tempfile::tempdir;

fn setup() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    vault::init(dir.path(), Some("test".into())).unwrap();
    dir
}

#[test]
fn new_and_list_and_show() {
    let dir = setup();
    let v = dir.path();

    let r = note::new(v, "Hello World", None).unwrap();
    assert_eq!(r.title, "Hello World");
    assert_eq!(r.slug, "hello-world");
    assert!(!r.id.is_empty());

    let list = note::list(v, ListParams::default()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].slug, "hello-world");

    let view = note::show(
        v,
        ShowParams {
            id_or_slug: "hello-world".into(),
            raw: false,
        },
    )
    .unwrap();
    assert_eq!(view.title, "Hello World");
    assert!(view.body.is_some());
    assert!(view.raw.is_none());
    assert!(view.body.as_deref().unwrap().starts_with("# Hello World"));

    let raw_view = note::show(
        v,
        ShowParams {
            id_or_slug: "hello-world".into(),
            raw: true,
        },
    )
    .unwrap();
    assert!(raw_view.body.is_none());
    assert!(raw_view.raw.as_deref().unwrap().starts_with("---"));
}

#[test]
fn update_title_body_append_archive() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "Original", None).unwrap();

    let r = note::update(
        v,
        UpdateParams {
            id_or_slug: "original".into(),
            title: Some("Renamed".into()),
            body: Some("Fresh body #rust with [[Link]]".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.title, "Renamed");
    assert_eq!(r.slug, "renamed");
    assert!(r.tags.contains(&"rust".to_string()));

    note::update(
        v,
        UpdateParams {
            id_or_slug: "renamed".into(),
            append: Some("extra line".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let view = note::show(
        v,
        ShowParams {
            id_or_slug: "renamed".into(),
            raw: false,
        },
    )
    .unwrap();
    let body = view.body.unwrap();
    assert!(body.contains("Fresh body"));
    assert!(body.contains("extra line"));
    assert!(body.starts_with("# Renamed"));

    note::update(
        v,
        UpdateParams {
            id_or_slug: "renamed".into(),
            archive: true,
            ..Default::default()
        },
    )
    .unwrap();
    let default_list = note::list(v, ListParams::default()).unwrap();
    assert_eq!(default_list.len(), 0);
    let arch = note::list(
        v,
        ListParams {
            only_archived: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(arch.len(), 1);
}

#[test]
fn update_requires_something() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "X", None).unwrap();
    let err = note::update(
        v,
        UpdateParams {
            id_or_slug: "x".into(),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, mem_core::CoreError::InvalidInput(_)));
}

#[test]
fn archive_and_unarchive_mutually_exclusive() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "X", None).unwrap();
    let err = note::update(
        v,
        UpdateParams {
            id_or_slug: "x".into(),
            archive: true,
            unarchive: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, mem_core::CoreError::InvalidInput(_)));
}

#[test]
fn show_missing() {
    let dir = setup();
    let v = dir.path();
    let err = note::show(
        v,
        ShowParams {
            id_or_slug: "nope".into(),
            raw: false,
        },
    )
    .unwrap_err();
    assert!(matches!(err, mem_core::CoreError::NoteNotFound(_)));
}

#[test]
fn search_finds_by_title_and_tag() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "Rust Notes", None).unwrap();
    note::update(
        v,
        UpdateParams {
            id_or_slug: "rust-notes".into(),
            body: Some("Learning #rust today".into()),
            ..Default::default()
        },
    )
    .unwrap();

    let hits = search::run(
        v,
        SearchParams {
            query: "Rust".into(),
            limit: None,
        },
    )
    .unwrap();
    assert!(!hits.is_empty());

    let tag_hits = search::run(
        v,
        SearchParams {
            query: "#rust".into(),
            limit: None,
        },
    )
    .unwrap();
    assert!(!tag_hits.is_empty());
}

#[test]
fn delete_removes_note() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "To Delete", None).unwrap();
    note::delete(v, "to-delete").unwrap();
    let list = note::list(v, ListParams::default()).unwrap();
    assert_eq!(list.len(), 0);
}

#[test]
fn new_with_body_inlines_content() {
    let dir = setup();
    let v = dir.path();
    let r = note::new(v, "With Body", Some("Initial text #rust")).unwrap();
    assert!(r.tags.contains(&"rust".to_string()));
    let view = note::show(
        v,
        ShowParams {
            id_or_slug: r.slug,
            raw: false,
        },
    )
    .unwrap();
    let body = view.body.unwrap();
    assert!(body.starts_with("# With Body"));
    assert!(body.contains("Initial text #rust"));
}

#[test]
fn list_filter_by_tag() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "Alpha", Some("hello #rust")).unwrap();
    note::new(v, "Beta", Some("hello #go")).unwrap();
    note::new(v, "Gamma", Some("no tags")).unwrap();

    let rust_only = note::list(
        v,
        ListParams {
            tag: Some("rust".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(rust_only.len(), 1);
    assert_eq!(rust_only[0].slug, "alpha");

    // accepts #-prefix too
    let with_hash = note::list(
        v,
        ListParams {
            tag: Some("#go".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(with_hash.len(), 1);
    assert_eq!(with_hash[0].slug, "beta");
}

#[test]
fn tags_list_returns_counts() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "A", Some("#rust #shared")).unwrap();
    note::new(v, "B", Some("#shared")).unwrap();

    let all = tags::list(v).unwrap();
    let shared = all.iter().find(|t| t.name == "shared").unwrap();
    assert_eq!(shared.note_count, 2);
    let rust = all.iter().find(|t| t.name == "rust").unwrap();
    assert_eq!(rust.note_count, 1);
}

#[test]
fn related_ranks_by_shared_tags() {
    let dir = setup();
    let v = dir.path();
    note::new(v, "Source", Some("#alpha #beta")).unwrap();
    note::new(v, "Both", Some("#alpha #beta")).unwrap();
    note::new(v, "One", Some("#alpha")).unwrap();
    note::new(v, "None", Some("#zebra")).unwrap();

    let related = note::related(
        v,
        RelatedParams {
            id_or_slug: "source".into(),
            limit: None,
        },
    )
    .unwrap();
    let slugs: Vec<&str> = related.iter().map(|n| n.slug.as_str()).collect();
    assert_eq!(slugs, vec!["both", "one"]);
}

#[test]
fn update_renames_file_on_slug_change() {
    let dir = setup();
    let v = dir.path();
    let created = note::new(v, "Old Title", None).unwrap();
    let old_path = std::path::PathBuf::from(&created.path);
    assert!(old_path.exists());

    let renamed = note::update(
        v,
        UpdateParams {
            id_or_slug: "old-title".into(),
            title: Some("Brand New".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(renamed.slug, "brand-new");
    assert_ne!(renamed.path, created.path);
    assert!(std::path::PathBuf::from(&renamed.path).exists());
    assert!(!old_path.exists(), "old file should be removed after rename");
}

#[test]
fn resolve_vault_from_cwd_and_env() {
    let dir = setup();
    let explicit = vault::resolve_vault(Some(dir.path())).unwrap();
    assert_eq!(explicit, dir.path());

    let missing = tempdir().unwrap();
    let err = vault::resolve_vault(Some(missing.path())).unwrap_err();
    assert!(matches!(err, mem_core::CoreError::NotAVault(_)));
}
