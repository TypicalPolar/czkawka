use super::*;
use crate::common::{IntDataSimilarImages, MAX_INT_DATA_SIMILAR_IMAGES, split_u64_into_i32s};

fn image(pixels: i32, size: u64) -> SingleMainListModel {
    let mut row = crate::test_common::get_main_list_model();
    let mut ints = vec![0; MAX_INT_DATA_SIMILAR_IMAGES];
    let (high, low) = split_u64_into_i32s(size);
    ints[IntDataSimilarImages::PixelCount as usize] = pixels;
    ints[IntDataSimilarImages::SizePart1 as usize] = high;
    ints[IntDataSimilarImages::SizePart2 as usize] = low;
    row.val_int = ModelRc::new(VecModel::from(ints));
    row
}

fn selection(images: &[(i32, u64)], tolerance: u32) -> Vec<bool> {
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let rows = std::iter::once(header).chain(images.iter().map(|&(pixels, size)| image(pixels, size))).collect::<Vec<_>>();
    let model = create_model_from_model_vec(&rows);
    let (checked, unchecked, result) = select_all_except_by_property(&model, ActiveTab::SimilarImages, Property::SizeNearHighestResolution(tolerance), true);
    assert_eq!(checked, images.len().saturating_sub(1) as u64);
    assert_eq!(unchecked, 0);
    assert!(!result.row_data(0).unwrap().checked);
    result.iter().skip(1).map(|row| row.checked).collect()
}

#[test]
fn tiny_resolution_difference_prefers_larger_file() {
    assert_eq!(selection(&[(1_000_000, 500), (1_006_009, 100)], 1), vec![false, true]);
    assert_eq!(selection(&[(1_006_009, 100), (1_000_000, 500)], 1), vec![true, false]);
}

#[test]
fn tolerance_boundary_is_inclusive_and_excludes_smaller_images() {
    assert_eq!(selection(&[(10_000, 100), (9_900, 200), (9_899, u64::MAX)], 1), vec![true, false, true]);
    assert_eq!(selection(&[(101, 100), (99, u64::MAX), (100, 200)], 1), vec![true, true, false]);
}

#[test]
fn candidates_are_compared_against_group_maximum() {
    assert_eq!(selection(&[(10_000, 100), (9_900, 200), (9_801, 300)], 1), vec![true, false, true]);
    assert_eq!(selection(&[(9_801, 300), (9_900, 200), (10_000, 100)], 1), vec![true, false, true]);
}

#[test]
fn zero_tolerance_requires_exact_resolution() {
    assert_eq!(selection(&[(10_000, 100), (9_999, u64::MAX), (10_000, 200)], 0), vec![true, true, false]);
}

#[test]
fn equal_sizes_prefer_resolution_then_first_row() {
    assert_eq!(selection(&[(9_900, u64::MAX), (10_000, u64::MAX), (10_000, u64::MAX)], 1), vec![true, false, true]);
}

#[test]
fn handles_zero_values_singletons_and_empty_groups() {
    assert_eq!(selection(&[(0, 0), (0, 1)], 1), vec![true, false]);
    assert_eq!(selection(&[(0, 0), (0, 0)], 1), vec![false, true]);
    assert_eq!(selection(&[(100, 1)], 1), vec![false]);
    assert_eq!(selection(&[], 1), Vec::<bool>::new());
}

#[test]
fn tolerance_applies_independently_and_counts_previous_selection() {
    let mut header = crate::test_common::get_main_list_model();
    header.header_row = true;
    let mut winner = image(9_900, 200);
    winner.checked = true;
    let rows = vec![header.clone(), image(10_000, 100), winner, header, image(100, 100), image(99, 200)];
    let (checked, unchecked, result) = select_all_except_by_property(&create_model_from_model_vec(&rows), ActiveTab::SimilarImages, Property::SizeNearHighestResolution(1), true);
    assert_eq!((checked, unchecked), (2, 1));
    assert_eq!(result.iter().map(|row| row.checked).collect::<Vec<_>>(), vec![false, true, false, false, true, false]);
}

#[test]
fn tolerance_setting_defaults_and_round_trips() {
    use crate::settings::model::BasicSettings;
    let mut settings: BasicSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(settings.selection_resolution_tolerance, 1);
    settings.selection_resolution_tolerance = 5;
    let json = serde_json::to_string(&settings).unwrap();
    let restored: BasicSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.selection_resolution_tolerance, 5);
}
