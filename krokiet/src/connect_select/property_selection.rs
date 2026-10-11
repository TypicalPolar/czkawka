use slint::{Model, ModelRc, VecModel};

use super::{Property, SelectionResult, extract_comparable_field, find_header_idx_and_deselect_all};
use crate::{ActiveTab, SingleMainListModel};

fn spared_item_index(rows: &[SingleMainListModel], active_tab: ActiveTab, property: Property, spare_max: bool) -> usize {
    let minimum_resolution = match property {
        Property::SizeNearHighestResolution(tolerance) => {
            let maximum = rows.iter().map(|row| extract_comparable_field(row, Property::Resolution, active_tab)).max().unwrap_or(0);
            (maximum * u128::from(100 - tolerance.min(100))).div_ceil(100)
        }
        _ => 0,
    };
    let mut extreme_val = if spare_max { 0u128 } else { u128::MAX };
    let mut extreme_idx = 0;
    for (idx, row) in rows.iter().enumerate() {
        if minimum_resolution > 0 && extract_comparable_field(row, Property::Resolution, active_tab) < minimum_resolution {
            continue;
        }
        let val = extract_comparable_field(row, property, active_tab);
        if spare_max && val > extreme_val || !spare_max && val < extreme_val {
            extreme_val = val;
            extreme_idx = idx;
        }
    }
    extreme_idx
}

// Selects all items in each group EXCEPT the one with the extreme property value.
// `spare_max: true`  → spares the biggest/newest/longest item (selects all others).
// `spare_max: false` → spares the smallest/oldest/shortest item (selects all others).
pub(super) fn select_all_except_by_property(model: &ModelRc<SingleMainListModel>, active_tab: ActiveTab, property: Property, spare_max: bool) -> SelectionResult {
    let mut checked_items = 0;
    let mut unchecked_items = 0;

    let is_header_mode = active_tab.get_is_header_mode();
    assert!(is_header_mode);

    let mut old_data = model.iter().collect::<Vec<_>>();
    // Compute selection deltas from the state before clearing the checkboxes.
    let prev_checked: Vec<bool> = old_data.iter().map(|m| m.checked).collect();
    let headers_idx = find_header_idx_and_deselect_all(&mut old_data);

    for i in 0..(headers_idx.len() - 1) {
        let group_start = headers_idx[i] + 1;
        let group_end = headers_idx[i + 1];

        let extreme_idx = group_start + spared_item_index(&old_data[group_start..group_end], active_tab, property, spare_max);

        for j in group_start..group_end {
            if j == extreme_idx {
                if prev_checked[j] {
                    unchecked_items += 1;
                }
                old_data[j].checked = false;
            } else {
                if !prev_checked[j] {
                    checked_items += 1;
                }
                old_data[j].checked = true;
            }
        }
    }

    (checked_items, unchecked_items, ModelRc::new(VecModel::from(old_data)))
}
