//! What the `door` guard reports.
//!
//! A capability's allowed locations, and everyone else held to not having
//! it. The expectations here were written before the guard existed.

#[test]
fn a_crate_not_on_the_list_is_reported() {
    // `other` is not the door and reaches through it anyway. This is the
    // case a prose "the only place ... is allowed" cannot report: it is
    // true when written and says nothing when a second place appears.
    //
    // The file is named because "this crate has the capability" is not
    // actionable on its own -- the fix is in a line, not in a crate.
    assert_eq!(
        twrit_tests::check("door-unlisted"),
        "file-io: other is not a std::fs door, and names it in other/src/lib.rs\n\
         1 writ, 1 rule, file-io: 1 violation (2 members)\n"
    );
}
