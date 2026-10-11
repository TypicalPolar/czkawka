use slint::{ModelRc, SharedString, VecModel};

use super::*;
use crate::common::{MAX_INT_DATA_DUPLICATE_FILES, MAX_STR_DATA_DUPLICATE_FILES, create_model_from_model_vec, split_u64_into_i32s};
use crate::test_common::get_model_vec;

// Builds a DuplicateFiles row with the given size encoded in val_int.
// IntDataDuplicateFiles layout: [ModDatePart1, ModDatePart2, SizePart1, SizePart2]
fn make_item_with_size(size: u64) -> SingleMainListModel {
    let (part1, part2) = split_u64_into_i32s(size);
    let ints: [i32; MAX_INT_DATA_DUPLICATE_FILES] = [0, 0, part1, part2];
    let strs: [SharedString; MAX_STR_DATA_DUPLICATE_FILES] = [SharedString::from(""), SharedString::from(""), SharedString::from(""), SharedString::from("")];
    SingleMainListModel {
        val_int: ModelRc::new(VecModel::from(ints.to_vec())),
        val_str: ModelRc::new(VecModel::from(strs.to_vec())),
        ..crate::test_common::get_main_list_model()
    }
}

// Builds a DuplicateFiles row with the given path and name in val_str.
// StrDataDuplicateFiles layout: [Size_display, Name, Path, ModDate]
fn make_item_with_path(path: &str, name: &str) -> SingleMainListModel {
    let ints: [i32; MAX_INT_DATA_DUPLICATE_FILES] = [0; MAX_INT_DATA_DUPLICATE_FILES];
    let strs: [SharedString; MAX_STR_DATA_DUPLICATE_FILES] = [SharedString::from(""), SharedString::from(name), SharedString::from(path), SharedString::from("")];
    SingleMainListModel {
        val_int: ModelRc::new(VecModel::from(ints.to_vec())),
        val_str: ModelRc::new(VecModel::from(strs.to_vec())),
        ..crate::test_common::get_main_list_model()
    }
}

#[test]
fn find_header_idx_returns_correct_indices_for_headers() {
    let mut model = get_model_vec(5);
    model[1].header_row = true;
    model[3].header_row = true;

    let header_indices = find_header_idx_and_deselect_all(&mut model);

    assert_eq!(header_indices, vec![1, 3, 5]);
}

#[test]
fn find_header_idx_marks_all_non_header_rows_as_unchecked() {
    let mut model = get_model_vec(5);
    for row in &mut model {
        row.checked = true;
    }
    model[1].header_row = true;

    find_header_idx_and_deselect_all(&mut model);

    assert!(!model[0].checked);
    assert!(model[1].checked);
    assert!(!model[2].checked);
    assert!(!model[3].checked);
    assert!(!model[4].checked);
}

#[test]
fn select_all_marks_all_non_header_rows_as_checked() {
    let mut model = get_model_vec(5);
    model[1].header_row = true;
    let model = create_model_from_model_vec(&model);

    let (checked_items, unchecked_items, new_model) = select_all(&model);

    assert_eq!(checked_items, 4);
    assert_eq!(unchecked_items, 0);
    assert!(new_model.row_data(0).unwrap().checked);
    assert!(!new_model.row_data(1).unwrap().checked);
    assert!(new_model.row_data(2).unwrap().checked);
    assert!(new_model.row_data(3).unwrap().checked);
    assert!(new_model.row_data(4).unwrap().checked);
}

#[test]
fn deselect_all_unmarks_all_rows_as_checked() {
    let mut model = get_model_vec(5);
    for row in &mut model {
        row.checked = true;
    }
    let model = create_model_from_model_vec(&model);

    let (checked_items, unchecked_items, new_model) = deselect_all(&model);

    assert_eq!(checked_items, 0);
    assert_eq!(unchecked_items, 5);
    assert!(!new_model.row_data(0).unwrap().checked);
    assert!(!new_model.row_data(1).unwrap().checked);
    assert!(!new_model.row_data(2).unwrap().checked);
    assert!(!new_model.row_data(3).unwrap().checked);
    assert!(!new_model.row_data(4).unwrap().checked);
}

#[test]
fn invert_selection_toggles_checked_state_for_non_header_rows() {
    let mut model = get_model_vec(5);
    model[0].checked = true;
    model[1].header_row = true;
    model[2].checked = false;
    let model = create_model_from_model_vec(&model);

    let (checked_items, unchecked_items, new_model) = invert_selection(&model);

    assert_eq!(checked_items, 3);
    assert_eq!(unchecked_items, 1);
    assert!(!new_model.row_data(0).unwrap().checked);
    assert!(!new_model.row_data(1).unwrap().checked);
    assert!(new_model.row_data(2).unwrap().checked);
    assert!(new_model.row_data(3).unwrap().checked);
    assert!(new_model.row_data(4).unwrap().checked);
}

#[test]
fn invert_selection_in_group_only_changes_groups_with_selection() {
    let mut model = get_model_vec(7);
    model[1].header_row = true; // Group 1 header
    // Group 1 (indices 0): item 0 - no selection initially
    model[2].header_row = true; // Group 2 header
    // Group 2 (indices 3,4,5,6): items 3,4,5,6 - item 3 selected
    model[3].checked = true;
    let model = create_model_from_model_vec(&model);

    let (checked_items, unchecked_items, new_model) = invert_selection_in_group(&model);

    // Group 1 has no selection, should remain unchanged
    assert!(!new_model.row_data(0).unwrap().checked);
    // Group 2 has selection, should invert: item 3 checked->unchecked, others unchecked->checked
    assert!(!new_model.row_data(3).unwrap().checked);
    assert!(new_model.row_data(4).unwrap().checked);
    assert!(new_model.row_data(5).unwrap().checked);
    assert!(new_model.row_data(6).unwrap().checked);
    // Verify counts: 3 items flipped from unchecked to checked, 1 from checked to unchecked
    assert_eq!(checked_items, 3);
    assert_eq!(unchecked_items, 1);
}

#[test]
fn invert_selection_in_group_does_nothing_when_no_groups_have_selection() {
    let mut model = get_model_vec(5);
    model[1].header_row = true;
    let model = create_model_from_model_vec(&model);

    let (checked_items, unchecked_items, new_model) = invert_selection_in_group(&model);

    assert_eq!(checked_items, 0);
    assert_eq!(unchecked_items, 0);
    assert!(!new_model.row_data(0).unwrap().checked);
    assert!(!new_model.row_data(2).unwrap().checked);
}

//  select_all_except_by_property

#[test]
fn select_all_except_biggest_spares_largest_item() {
    // Layout: [header, small(100), medium(200), large(300)]
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let items = vec![header, make_item_with_size(100), make_item_with_size(200), make_item_with_size(300)];
    let model = create_model_from_model_vec(&items);

    let (checked_items, unchecked_items, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::Size, true);

    assert_eq!(checked_items, 2);
    assert_eq!(unchecked_items, 0);
    assert!(new_model.row_data(1).unwrap().checked); // 100 - selected
    assert!(new_model.row_data(2).unwrap().checked); // 200 - selected
    assert!(!new_model.row_data(3).unwrap().checked); // 300 - spared (biggest)
}

#[test]
fn select_all_except_smallest_spares_smallest_item() {
    // Layout: [header, small(100), medium(200), large(300)]
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let items = vec![header, make_item_with_size(100), make_item_with_size(200), make_item_with_size(300)];
    let model = create_model_from_model_vec(&items);

    let (checked_items, unchecked_items, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::Size, false);

    assert_eq!(checked_items, 2);
    assert_eq!(unchecked_items, 0);
    assert!(!new_model.row_data(1).unwrap().checked); // 100 - spared (smallest)
    assert!(new_model.row_data(2).unwrap().checked); // 200 - selected
    assert!(new_model.row_data(3).unwrap().checked); // 300 - selected
}

#[test]
fn select_all_except_operates_independently_per_group() {
    // Group 1: [header, 100, 300]  - spare 300 (biggest) / spare 100 (smallest)
    // Group 2: [header, 50, 150]   - spare 150 (biggest) / spare 50  (smallest)
    let mut h1 = crate::test_common::get_main_list_model();
    h1.header_row = true;
    let mut h2 = crate::test_common::get_main_list_model();
    h2.header_row = true;
    let items = vec![
        h1,
        make_item_with_size(100),
        make_item_with_size(300),
        h2,
        make_item_with_size(50),
        make_item_with_size(150),
    ];
    let model = create_model_from_model_vec(&items);

    let (_checked, _unchecked, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::Size, true);

    assert!(new_model.row_data(1).unwrap().checked); // 100 - selected
    assert!(!new_model.row_data(2).unwrap().checked); // 300 - spared (biggest in group 1)
    assert!(new_model.row_data(4).unwrap().checked); // 50 - selected
    assert!(!new_model.row_data(5).unwrap().checked); // 150 - spared (biggest in group 2)
}

#[test]
fn select_all_except_delta_accounts_for_previously_checked_items() {
    // Items 1 and 2 are pre-checked; biggest (300) will be spared → item 1 gets unchecked again.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let mut item_small = make_item_with_size(100);
    item_small.checked = true; // pre-checked
    let mut item_medium = make_item_with_size(200);
    item_medium.checked = true; // pre-checked
    let item_large = make_item_with_size(300);
    let items = vec![header, item_small, item_medium, item_large];
    let model = create_model_from_model_vec(&items);

    let (checked_items, unchecked_items, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::Size, true);

    // item_small was already checked → not counted as newly checked
    // item_medium was already checked → not counted as newly checked
    // item_large was unchecked and will be spared → no change counted for it
    assert_eq!(checked_items, 0);
    // item_large was not checked, so it contributes 0 unchecked_items; medium was checked but stays checked
    // Actually item_large remains unchecked (spared), so unchecked_items should be 0
    assert_eq!(unchecked_items, 0);
    assert!(new_model.row_data(1).unwrap().checked);
    assert!(new_model.row_data(2).unwrap().checked);
    assert!(!new_model.row_data(3).unwrap().checked);
}

#[test]
fn select_all_except_counts_unchecked_when_spared_item_was_previously_checked() {
    // Only item_large is pre-checked; it will be spared → unchecked_items += 1.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let item_small = make_item_with_size(100);
    let item_medium = make_item_with_size(200);
    let mut item_large = make_item_with_size(300);
    item_large.checked = true; // pre-checked, but will be spared
    let items = vec![header, item_small, item_medium, item_large];
    let model = create_model_from_model_vec(&items);

    let (checked_items, unchecked_items, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::Size, true);

    assert_eq!(checked_items, 2); // small and medium go from unchecked → checked
    assert_eq!(unchecked_items, 1); // large goes from checked → unchecked (spared)
    assert!(new_model.row_data(1).unwrap().checked);
    assert!(new_model.row_data(2).unwrap().checked);
    assert!(!new_model.row_data(3).unwrap().checked);
}

#[test]
fn select_all_except_longest_path_spares_item_with_longest_full_path() {
    // StrDataDuplicateFiles: [Size_display, Name, Path, ModDate]
    // Primary sort key: directory path length; secondary: filename length.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    // short: "/a" (dir=2) + "x.jpg"    → key (2, 5)
    // medium: "/ab" (dir=3) + "x.jpg"  → key (3, 5)
    // long: "/abc" (dir=4) + "x.jpg"   → key (4, 5)
    let items = vec![
        header,
        make_item_with_path("/a", "x.jpg"),
        make_item_with_path("/ab", "x.jpg"),
        make_item_with_path("/abc", "x.jpg"),
    ];
    let model = create_model_from_model_vec(&items);

    let (_checked, _unchecked, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::PathLength, true);

    assert!(new_model.row_data(1).unwrap().checked); // short - selected
    assert!(new_model.row_data(2).unwrap().checked); // medium - selected
    assert!(!new_model.row_data(3).unwrap().checked); // long - spared (longest)
}

#[test]
fn select_all_except_shortest_path_spares_item_with_shortest_full_path() {
    // Primary sort key: directory path length; secondary: filename length.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let items = vec![
        header,
        make_item_with_path("/a", "x.jpg"),
        make_item_with_path("/ab", "x.jpg"),
        make_item_with_path("/abc", "x.jpg"),
    ];
    let model = create_model_from_model_vec(&items);

    let (_checked, _unchecked, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::PathLength, false);

    assert!(!new_model.row_data(1).unwrap().checked); // short - spared (shortest)
    assert!(new_model.row_data(2).unwrap().checked); // medium - selected
    assert!(new_model.row_data(3).unwrap().checked); // long - selected
}

#[test]
fn select_all_except_shortest_path_uses_filename_to_break_ties_in_same_directory() {
    // All files share the same directory; path length differs only by filename.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    // IMG_0001.JPG (13 chars) < IMG_0001-1.JPG (15 chars) < IMG_0001-2.JPG (15 chars)
    let items = vec![
        header,
        make_item_with_path("/photos", "IMG_0001.JPG"),
        make_item_with_path("/photos", "IMG_0001-1.JPG"),
        make_item_with_path("/photos", "IMG_0001-2.JPG"),
    ];
    let model = create_model_from_model_vec(&items);

    let (_checked, _unchecked, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::PathLength, false);

    assert!(!new_model.row_data(1).unwrap().checked); // IMG_0001.JPG - spared (shortest)
    assert!(new_model.row_data(2).unwrap().checked);
    assert!(new_model.row_data(3).unwrap().checked);
}

#[test]
fn select_all_except_shortest_path_prefers_directory_length_over_filename_length() {
    // File in a shorter directory but with a long name must still be spared
    // over a file in a longer directory with a short name.
    // Before the fix, summing lengths could invert this result.
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    // short_dir + long_name  → dir=3, name=20 → was sum=23, now key=(3<<32)|20
    // long_dir  + short_name → dir=20, name=3  → was sum=23, now key=(20<<32)|3
    let items = vec![
        header,
        make_item_with_path("/sd", "a_very_long_filename.jpg"),   // dir shorter → spared
        make_item_with_path("/a/much/longer/directory", "b.jpg"), // dir longer → selected
    ];
    let model = create_model_from_model_vec(&items);

    let (_checked, _unchecked, new_model) = select_all_except_by_property(&model, ActiveTab::DuplicateFiles, Property::PathLength, false);

    assert!(!new_model.row_data(1).unwrap().checked); // shorter dir - spared
    assert!(new_model.row_data(2).unwrap().checked); // longer dir - selected
}

#[test]
fn resolution_then_size_spares_one_best_image_per_group() {
    use crate::common::{IntDataSimilarImages, MAX_INT_DATA_SIMILAR_IMAGES};

    let image = |pixels, size| {
        let mut row = crate::test_common::get_main_list_model();
        let mut ints = vec![0; MAX_INT_DATA_SIMILAR_IMAGES];
        let (high, low) = split_u64_into_i32s(size);
        ints[IntDataSimilarImages::PixelCount as usize] = pixels;
        ints[IntDataSimilarImages::SizePart1 as usize] = high;
        ints[IntDataSimilarImages::SizePart2 as usize] = low;
        row.val_int = ModelRc::new(VecModel::from(ints));
        row
    };
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let mut previously_selected = image(200, 1u64 << 40);
    previously_selected.checked = true;
    let rows = vec![
        header.clone(),
        image(100, u64::MAX),
        image(200, 100),
        previously_selected,
        image(200, 1u64 << 40),
        header,
        image(0, 0),
        image(0, 1),
    ];
    let model = create_model_from_model_vec(&rows);
    let (checked, unchecked, result) = select_all_except_by_property(&model, ActiveTab::SimilarImages, Property::ResolutionThenSize, true);

    assert_eq!(checked, 4);
    assert_eq!(unchecked, 1);
    assert_eq!(
        result.iter().map(|row| row.checked).collect::<Vec<_>>(),
        vec![false, true, true, false, true, false, true, false]
    );
}
