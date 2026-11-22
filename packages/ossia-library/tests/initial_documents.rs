use ossia_library::Library;

#[test]
fn initial_documents_marker_lives_only_for_new_libraries() {
    let root = tempfile::tempdir().unwrap();
    let library = Library::open(root.path()).unwrap();
    assert!(library.initial_documents_pending().unwrap());

    library.finish_initial_documents().unwrap();
    assert!(!library.initial_documents_pending().unwrap());
    library.finish_initial_documents().unwrap();

    drop(library);
    let library = Library::open(root.path()).unwrap();
    assert!(!library.initial_documents_pending().unwrap());
}
