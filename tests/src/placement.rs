//! What the `placement` guard reports.

#[test]
fn clean_placement_passes() {
    assert_eq!(
        twrit_tests::check("placement-clean"),
        "1 writ, 1 rule, placement-clean: ok (2 files)\n"
    );
}

#[test]
fn stray_file_is_reported() {
    assert_eq!(
        twrit_tests::check("placement-stray"),
        "placement-stray: docs/unexpected.tmt: unlisted file matches no allowed pattern\n\
         1 writ, 1 rule, placement-stray: 1 violation (3 files)\n"
    );
}
