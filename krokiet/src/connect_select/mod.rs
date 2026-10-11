pub(crate) mod custom_select;
mod property_selection;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use log::error;
use property_selection::select_all_except_by_property;
use regex::Regex;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::common::{connect_i32_into_u64, create_model_from_model_vec};
use crate::connect_row_selection::checker::change_number_of_enabled_items;
use crate::connect_translation::translate_select_mode;
use crate::settings::model::{SavedCustomSelectColumnState, SavedCustomSelectTabState};
use crate::settings::{get_custom_select_state_file, load_data_from_file, save_data_to_file};
use crate::shared_models::SharedModels;
use crate::{
    ActiveTab, Callabler, CustomSelectColumnModel, GuiState, MainWindow, SelectItemsCustomColumnsRequest, SelectMode, SelectModel, Settings, SingleMainListModel,
    UpdateCustomSelectColumnRequest,
};

type SelectionResult = (u64, u64, ModelRc<SingleMainListModel>);

// TODO optimize this, not sure if it is possible to not copy entire model to just select item
// https://github.com/slint-ui/slint/discussions/4595
pub(crate) fn connect_select(app: &MainWindow, shared_models: &Arc<Mutex<SharedModels>>) {
    set_select_buttons(app);

    let a = app.as_weak();
    app.global::<Callabler>().on_update_select_buttons(move || {
        let app = a.upgrade().expect("Failed to upgrade app :(");
        set_select_buttons(&app);
    });

    let shared_models = shared_models.clone();
    let a = app.as_weak();
    app.global::<Callabler>().on_select_items(move |select_mode| {
        let app = a.upgrade().expect("Failed to upgrade app :(");
        let active_tab = app.global::<GuiState>().get_active_tab();
        let current_model = active_tab.get_tool_model(&app);

        let (checked_items, unchecked_items, new_model) = match select_mode {
            SelectMode::SelectAll => select_all(&current_model),
            SelectMode::UnselectAll => deselect_all(&current_model),
            SelectMode::InvertSelection => invert_selection(&current_model),
            SelectMode::InvertSelectionInGroup => invert_selection_in_group(&current_model),
            SelectMode::SelectTheBiggestSize => select_by_property(&current_model, active_tab, Property::Size, true),
            SelectMode::SelectTheSmallestSize => select_by_property(&current_model, active_tab, Property::Size, false),
            SelectMode::SelectTheBiggestResolution => select_by_property(&current_model, active_tab, Property::Resolution, true),
            SelectMode::SelectTheSmallestResolution => select_by_property(&current_model, active_tab, Property::Resolution, false),
            SelectMode::SelectNewest => select_by_property(&current_model, active_tab, Property::Date, true),
            SelectMode::SelectOldest => select_by_property(&current_model, active_tab, Property::Date, false),
            SelectMode::SelectShortestPath => select_by_property(&current_model, active_tab, Property::PathLength, false),
            SelectMode::SelectLongestPath => select_by_property(&current_model, active_tab, Property::PathLength, true),
            SelectMode::SelectAllExceptBiggestSize => select_all_except_by_property(&current_model, active_tab, Property::Size, true),
            SelectMode::SelectAllExceptSmallestSize => select_all_except_by_property(&current_model, active_tab, Property::Size, false),
            SelectMode::SelectAllExceptBiggestResolution => select_all_except_by_property(&current_model, active_tab, Property::Resolution, true),
            SelectMode::SelectAllExceptBiggestResolutionAndSize => select_all_except_by_property(&current_model, active_tab, Property::ResolutionThenSize, true),
            SelectMode::SelectAllExceptLargestFileNearHighestResolution => select_all_except_by_property(
                &current_model,
                active_tab,
                Property::SizeNearHighestResolution(app.global::<Settings>().get_selection_resolution_tolerance().clamp(0, 100) as u32),
                true,
            ),
            SelectMode::SelectAllExceptSmallestResolution => select_all_except_by_property(&current_model, active_tab, Property::Resolution, false),
            SelectMode::SelectAllExceptNewest => select_all_except_by_property(&current_model, active_tab, Property::Date, true),
            SelectMode::SelectAllExceptOldest => select_all_except_by_property(&current_model, active_tab, Property::Date, false),
            SelectMode::SelectAllExceptLongestPath => select_all_except_by_property(&current_model, active_tab, Property::PathLength, true),
            SelectMode::SelectAllExceptShortestPath => select_all_except_by_property(&current_model, active_tab, Property::PathLength, false),

            SelectMode::SelectCustom => return,
        };
        active_tab.set_tool_model(&app, new_model);
        change_number_of_enabled_items(&app, active_tab, checked_items as i64 - unchecked_items as i64);
    });

    app.global::<Callabler>().on_validate_regex(|regex_str| {
        if regex_str.is_empty() {
            return true;
        }
        Regex::new(regex_str.as_str()).is_ok()
    });

    let a = app.as_weak();
    app.global::<Callabler>().on_populate_custom_select_columns(move || {
        let app = a.upgrade().expect("Failed to upgrade app :(");
        let active_tab = app.global::<GuiState>().get_active_tab();
        let save_restore = app.global::<Settings>().get_popup_custom_select_save_restore();

        let mut columns = custom_select::build_custom_select_columns(active_tab);

        if save_restore {
            let tab_key = format!("{active_tab:?}");
            let saved_states: BTreeMap<String, SavedCustomSelectTabState> = load_data_from_file(get_custom_select_state_file()).unwrap_or_default();
            if let Some(saved) = saved_states.get(&tab_key) {
                for (col, saved_col) in columns.iter_mut().zip(saved.columns.iter()) {
                    col.enabled = saved_col.enabled;
                    col.filter_value = SharedString::from(saved_col.filter_value.as_str());
                }
                app.global::<GuiState>().set_custom_select_restored_case_sensitive(saved.case_sensitive);
                app.global::<GuiState>().set_custom_select_restored_leave_one_in_group(saved.leave_one_in_group);
            } else {
                app.global::<GuiState>().set_custom_select_restored_case_sensitive(false);
                app.global::<GuiState>().set_custom_select_restored_leave_one_in_group(true);
            }
        }

        app.global::<GuiState>().set_custom_select_columns(create_model_from_model_vec(&columns));
    });

    let a = app.as_weak();
    app.global::<Callabler>().on_update_custom_select_column(move |request| {
        let UpdateCustomSelectColumnRequest { col_idx, enabled, filter_value } = request;
        let app = a.upgrade().expect("Failed to upgrade app :(");
        let model = app.global::<GuiState>().get_custom_select_columns();
        let idx = col_idx as usize;
        let mut col = model
            .row_data(idx)
            .unwrap_or_else(|| panic!("Custom select column idx={idx} out of bounds (row_count={})", model.row_count()));
        col.enabled = enabled;
        col.filter_value = filter_value;
        model.set_row_data(idx, col);
    });

    let a = app.as_weak();
    app.global::<Callabler>().on_select_items_custom_columns(move |request| {
        let SelectItemsCustomColumnsRequest {
            select_mode,
            case_sensitive,
            leave_one_in_group,
            save_restore,
        } = request;
        let app = a.upgrade().expect("Failed to upgrade app :(");
        let active_tab = app.global::<GuiState>().get_active_tab();
        let current_model = active_tab.get_tool_model(&app);
        let columns: Vec<CustomSelectColumnModel> = app.global::<GuiState>().get_custom_select_columns().iter().collect();

        let leave_one_in_group = leave_one_in_group && (active_tab.get_is_header_mode() && !shared_models.lock().expect("Lock poisoned").get_use_reference_folders(active_tab));

        if save_restore {
            let tab_key = format!("{active_tab:?}");
            let mut saved_states: BTreeMap<String, SavedCustomSelectTabState> = load_data_from_file(get_custom_select_state_file()).unwrap_or_default();
            let saved_columns = columns
                .iter()
                .map(|c| SavedCustomSelectColumnState {
                    enabled: c.enabled,
                    filter_value: c.filter_value.to_string(),
                })
                .collect();
            saved_states.insert(
                tab_key,
                SavedCustomSelectTabState {
                    case_sensitive,
                    leave_one_in_group,
                    columns: saved_columns,
                },
            );
            if let Err(e) = save_data_to_file(get_custom_select_state_file(), &saved_states) {
                error!("Failed to save custom select state: {e}");
            }
        }

        let (checked_items, unchecked_items, new_model) =
            custom_select::select_custom_columns(&current_model, active_tab, select_mode, &columns, case_sensitive, leave_one_in_group);
        active_tab.set_tool_model(&app, new_model);
        change_number_of_enabled_items(&app, active_tab, checked_items as i64 - unchecked_items as i64);
    });
}

#[derive(Clone, Copy)]
enum Property {
    Size,
    Date,
    PathLength,
    Resolution,
    ResolutionThenSize,
    SizeNearHighestResolution(u32),
}

pub(crate) fn set_select_buttons(app: &MainWindow) {
    let active_tab = app.global::<GuiState>().get_active_tab();
    let settings = app.global::<Settings>();
    let mut base_buttons = vec![SelectMode::SelectCustom, SelectMode::SelectAll, SelectMode::UnselectAll, SelectMode::InvertSelection];

    let additional_buttons = match active_tab {
        ActiveTab::DuplicateFiles | ActiveTab::SimilarVideos | ActiveTab::SimilarMusic => vec![
            (SelectMode::InvertSelectionInGroup, true),
            (SelectMode::SelectOldest, settings.get_select_show_oldest()),
            (SelectMode::SelectNewest, settings.get_select_show_newest()),
            (SelectMode::SelectTheSmallestSize, settings.get_select_show_smallest_size()),
            (SelectMode::SelectTheBiggestSize, settings.get_select_show_biggest_size()),
            (SelectMode::SelectShortestPath, settings.get_select_show_shortest_path()),
            (SelectMode::SelectLongestPath, settings.get_select_show_longest_path()),
            (SelectMode::SelectAllExceptOldest, settings.get_select_show_except_oldest()),
            (SelectMode::SelectAllExceptNewest, settings.get_select_show_except_newest()),
            (SelectMode::SelectAllExceptSmallestSize, settings.get_select_show_except_smallest_size()),
            (SelectMode::SelectAllExceptBiggestSize, settings.get_select_show_except_biggest_size()),
            (SelectMode::SelectAllExceptShortestPath, settings.get_select_show_except_shortest_path()),
            (SelectMode::SelectAllExceptLongestPath, settings.get_select_show_except_longest_path()),
        ],
        ActiveTab::SimilarImages => vec![
            (SelectMode::InvertSelectionInGroup, true),
            (SelectMode::SelectOldest, settings.get_select_show_oldest()),
            (SelectMode::SelectNewest, settings.get_select_show_newest()),
            (SelectMode::SelectTheSmallestSize, settings.get_select_show_smallest_size()),
            (SelectMode::SelectTheBiggestSize, settings.get_select_show_biggest_size()),
            (SelectMode::SelectTheSmallestResolution, settings.get_select_show_smallest_resolution()),
            (SelectMode::SelectTheBiggestResolution, settings.get_select_show_biggest_resolution()),
            (SelectMode::SelectShortestPath, settings.get_select_show_shortest_path()),
            (SelectMode::SelectLongestPath, settings.get_select_show_longest_path()),
            (SelectMode::SelectAllExceptOldest, settings.get_select_show_except_oldest()),
            (SelectMode::SelectAllExceptNewest, settings.get_select_show_except_newest()),
            (SelectMode::SelectAllExceptSmallestSize, settings.get_select_show_except_smallest_size()),
            (SelectMode::SelectAllExceptBiggestSize, settings.get_select_show_except_biggest_size()),
            (SelectMode::SelectAllExceptSmallestResolution, settings.get_select_show_except_smallest_resolution()),
            (SelectMode::SelectAllExceptBiggestResolution, settings.get_select_show_except_biggest_resolution()),
            (SelectMode::SelectAllExceptBiggestResolutionAndSize, settings.get_select_show_except_biggest_resolution()),
            (
                SelectMode::SelectAllExceptLargestFileNearHighestResolution,
                settings.get_select_show_except_biggest_resolution(),
            ),
            (SelectMode::SelectAllExceptShortestPath, settings.get_select_show_except_shortest_path()),
            (SelectMode::SelectAllExceptLongestPath, settings.get_select_show_except_longest_path()),
        ],
        ActiveTab::EmptyFolders
        | ActiveTab::BigFiles
        | ActiveTab::EmptyFiles
        | ActiveTab::TemporaryFiles
        | ActiveTab::InvalidSymlinks
        | ActiveTab::BrokenFiles
        | ActiveTab::BadExtensions
        | ActiveTab::BadNames
        | ActiveTab::ExifRemover
        | ActiveTab::VideoOptimizer
        | ActiveTab::Settings
        | ActiveTab::About => Vec::new(),
    };

    base_buttons.extend(additional_buttons.into_iter().filter_map(|(mode, enabled)| enabled.then_some(mode)));
    base_buttons.reverse();

    let new_select_model = base_buttons
        .into_iter()
        .map(|e| SelectModel {
            name: translate_select_mode(e),
            data: e,
        })
        .collect::<Vec<_>>();

    app.global::<GuiState>().set_select_results_list(ModelRc::new(VecModel::from(new_select_model)));
}

fn extract_comparable_field(model: &SingleMainListModel, property: Property, active_tab: ActiveTab) -> u128 {
    let mut val_ints = model.val_int.iter();
    u128::from(match property {
        Property::SizeNearHighestResolution(_) => {
            return (extract_comparable_field(model, Property::Size, active_tab) << 64) | extract_comparable_field(model, Property::Resolution, active_tab);
        }
        Property::ResolutionThenSize => {
            return (extract_comparable_field(model, Property::Resolution, active_tab) << 64) | extract_comparable_field(model, Property::Size, active_tab);
        }
        Property::Size => {
            let high = val_ints.nth(active_tab.get_int_size_idx()).expect("can find file size property");
            let low = val_ints.next().expect("can find file size property");
            connect_i32_into_u64(high, low)
        }
        Property::Date => {
            let high = val_ints.nth(active_tab.get_int_modification_date_idx()).expect("can find file last modified property");
            let low = val_ints.next().expect("can find file last modified property");
            connect_i32_into_u64(high, low)
        }
        Property::PathLength => {
            let path_len = model.val_str.iter().nth(active_tab.get_str_path_idx()).expect("can find file path property").len();
            let name_len = model.val_str.iter().nth(active_tab.get_str_name_idx()).expect("can find file name property").len();
            // Primary key: directory path length; secondary key: filename length.
            // Packed into a single u64 so the existing comparison infrastructure works unchanged.
            ((path_len as u64) << 32) | (name_len as u64)
        }
        Property::Resolution => val_ints.nth(active_tab.get_int_pixel_count_idx()).expect("can find pixel count proerty") as u64,
    })
}

fn select_by_property(model: &ModelRc<SingleMainListModel>, active_tab: ActiveTab, property: Property, select_max: bool) -> SelectionResult {
    let mut checked_items = 0;

    let is_header_mode = active_tab.get_is_header_mode();
    assert!(is_header_mode); // non header modes not really have reason to use this function

    let mut old_data = model.iter().collect::<Vec<_>>();
    let headers_idx = find_header_idx_and_deselect_all(&mut old_data);
    if select_max {
        for i in 0..(headers_idx.len() - 1) {
            let mut max_item = 0;
            let mut max_item_idx = 1;
            #[expect(clippy::needless_range_loop)]
            for j in (headers_idx[i] + 1)..headers_idx[i + 1] {
                let item = extract_comparable_field(&old_data[j], property, active_tab);
                if item > max_item {
                    max_item = item;
                    max_item_idx = j;
                }
            }
            if !old_data[max_item_idx].checked {
                checked_items += 1;
            }
            old_data[max_item_idx].checked = true;
        }
    } else {
        for i in 0..(headers_idx.len() - 1) {
            let mut min_item = u128::MAX;
            let mut min_item_idx = 1;
            #[expect(clippy::needless_range_loop)]
            for j in (headers_idx[i] + 1)..headers_idx[i + 1] {
                let item = extract_comparable_field(&old_data[j], property, active_tab);
                if item < min_item {
                    min_item = item;
                    min_item_idx = j;
                }
            }
            if !old_data[min_item_idx].checked {
                checked_items += 1;
            }
            old_data[min_item_idx].checked = true;
        }
    }

    (checked_items, 0, ModelRc::new(VecModel::from(old_data)))
}

fn select_all(model: &ModelRc<SingleMainListModel>) -> SelectionResult {
    let mut checked_items = 0;
    let mut old_data = model.iter().collect::<Vec<_>>();
    for x in &mut old_data {
        if !x.header_row {
            if !x.checked {
                checked_items += 1;
            }
            x.checked = true;
        }
    }
    (checked_items, 0, ModelRc::new(VecModel::from(old_data)))
}

fn deselect_all(model: &ModelRc<SingleMainListModel>) -> SelectionResult {
    let mut unchecked_items = 0;
    let mut old_data = model.iter().collect::<Vec<_>>();
    for x in &mut old_data {
        if x.checked {
            unchecked_items += 1;
        }
        x.checked = false;
    }
    (0, unchecked_items, ModelRc::new(VecModel::from(old_data)))
}

fn invert_selection(model: &ModelRc<SingleMainListModel>) -> SelectionResult {
    let mut checked_items = 0;
    let mut unchecked_items = 0;
    let mut old_data = model.iter().collect::<Vec<_>>();
    for x in &mut old_data {
        if !x.header_row {
            if x.checked {
                unchecked_items += 1;
            } else {
                checked_items += 1;
            }

            x.checked = !x.checked;
        }
    }
    (checked_items, unchecked_items, ModelRc::new(VecModel::from(old_data)))
}

fn invert_selection_in_group(model: &ModelRc<SingleMainListModel>) -> SelectionResult {
    let mut checked_items = 0;
    let mut unchecked_items = 0;
    let mut old_data = model.iter().collect::<Vec<_>>();

    let header_idx: Vec<usize> = old_data
        .iter()
        .enumerate()
        .filter_map(|(idx, m)| if m.header_row { Some(idx) } else { None })
        .chain(std::iter::once(old_data.len()))
        .collect();

    for group_idx in 0..header_idx.len() - 1 {
        let group_start = header_idx[group_idx] + 1;
        let group_end = header_idx[group_idx + 1];

        let has_selection_in_group = old_data[group_start..group_end].iter().any(|x| x.checked);

        if !has_selection_in_group {
            continue;
        }

        for i in group_start..group_end {
            if old_data[i].checked {
                unchecked_items += 1;
            } else {
                checked_items += 1;
            }
            old_data[i].checked = !old_data[i].checked;
        }
    }

    (checked_items, unchecked_items, ModelRc::new(VecModel::from(old_data)))
}

fn find_header_idx_and_deselect_all(old_data: &mut [SingleMainListModel]) -> Vec<usize> {
    let mut header_idx = old_data
        .iter()
        .enumerate()
        .filter_map(|(idx, m)| if m.header_row { Some(idx) } else { None })
        .collect::<Vec<_>>();
    header_idx.push(old_data.len());

    for x in old_data.iter_mut() {
        if !x.header_row {
            x.checked = false;
        }
    }
    header_idx
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tolerance_tests;
