use crate::tab_bar::{line::tab_separator, LinePart};
use ansi_term::{ANSIString, ANSIStrings};
use unicode_width::UnicodeWidthStr;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;

fn cursors<'a>(
    focused_client_slots: &'a [usize],
    multiplayer_colors: MultiplayerColors,
) -> (Vec<ANSIString<'a>>, usize) {
    let mut len = 0;
    let mut cursors = vec![];
    for display_slot in focused_client_slots.iter() {
        if let Some(color) = client_slot_to_colors(*display_slot, multiplayer_colors) {
            cursors.push(style!(color.1, color.0).paint(" "));
            len += 1;
        }
    }
    (cursors, len)
}

pub fn render_tab(
    text: String,
    tab: &TabInfo,
    is_alternate_tab: bool,
    is_hovered: bool,
    palette: Styling,
    separator: &str,
    mode_info: &ModeInfo,
) -> LinePart {
    let dimmed = mode_info.session_ascended == Some(true) || mode_info.session_dimmed == Some(true);
    let is_ancestor =
        mode_info.session_dimmed == Some(true) && mode_info.session_ascended != Some(true);
    let ancestor_style = is_ancestor.then(|| palette.ancestor_ribbon_style(tab.active));
    let focused_clients = tab.other_focused_client_slots.as_slice();
    let separator_width = separator.width();

    let alternate_tab_color = if is_alternate_tab {
        palette.ribbon_unselected.emphasis_1
    } else {
        palette.ribbon_unselected.background
    };
    let background_color = if tab.active {
        palette.ribbon_selected.background
    } else if is_hovered {
        palette.ribbon_unselected.emphasis_1
    } else if is_alternate_tab {
        alternate_tab_color
    } else {
        palette.ribbon_unselected.background
    };
    let foreground_color = if tab.is_flashing_bell {
        if tab.active {
            palette.ribbon_selected.emphasis_3
        } else {
            palette.ribbon_unselected.emphasis_3
        }
    } else if tab.active {
        palette.ribbon_selected.base
    } else {
        palette.ribbon_unselected.base
    };

    let separator_fill_color = palette.text_unselected.background;
    let background_color = if let Some(ancestor_style) = ancestor_style {
        ancestor_style.background
    } else if dimmed {
        palette.ribbon_unselected.background
    } else {
        background_color
    };
    let text_style = if let Some(ancestor_style) = ancestor_style {
        style!(ancestor_style.base, background_color).italic()
    } else if dimmed {
        style!(palette.ribbon_unselected.base, background_color).italic()
    } else {
        style!(foreground_color, background_color).bold()
    };
    let left_separator = style!(separator_fill_color, background_color).paint(separator);
    let right_separator_style = style!(background_color, separator_fill_color);
    let mut tab_text_len = text.width() + (separator_width * 2) + 2;
    let tab_styled_text = text_style.paint(format!(" {} ", text));

    let right_separator = right_separator_style.paint(separator);
    let tab_styled_text = if !focused_clients.is_empty() {
        let (cursor_section, extra_length) =
            cursors(focused_clients, palette.multiplayer_user_colors);
        tab_text_len += extra_length + 2;
        let mut s = String::new();
        let cursor_beginning = text_style.paint("[").to_string();
        let cursor_section = ANSIStrings(&cursor_section).to_string();
        let cursor_end = text_style.paint("]").to_string();
        s.push_str(&left_separator.to_string());
        s.push_str(&tab_styled_text.to_string());
        s.push_str(&cursor_beginning);
        s.push_str(&cursor_section);
        s.push_str(&cursor_end);
        s.push_str(&right_separator.to_string());
        s
    } else {
        ANSIStrings(&[left_separator, tab_styled_text, right_separator]).to_string()
    };

    LinePart {
        part: tab_styled_text,
        len: tab_text_len,
        tab_index: Some(tab.position),
    }
}

pub fn tab_style(
    mut tabname: String,
    tab: &TabInfo,
    mut is_alternate_tab: bool,
    is_hovered: bool,
    palette: Styling,
    capabilities: PluginCapabilities,
    mode_info: &ModeInfo,
) -> LinePart {
    let separator = tab_separator(capabilities);

    if tab.is_fullscreen_active {
        tabname.push_str(" (FULLSCREEN)");
    } else if tab.is_sync_panes_active {
        tabname.push_str(" (SYNC)");
    }
    if tab.has_bell_notification || tab.is_flashing_bell {
        tabname.push_str(" [!]");
    }
    if !capabilities.arrow_fonts {
        is_alternate_tab = false;
    }

    render_tab(
        tabname,
        tab,
        is_alternate_tab,
        is_hovered,
        palette,
        separator,
        mode_info,
    )
}

pub(crate) fn get_tab_to_focus(
    tab_line: &[LinePart],
    active_tab_idx: usize,
    mouse_click_col: usize,
) -> Option<usize> {
    let clicked_line_part = get_clicked_line_part(tab_line, mouse_click_col)?;
    let clicked_tab_idx = clicked_line_part.tab_index?;
    let clicked_tab_idx = clicked_tab_idx + 1;
    if clicked_tab_idx != active_tab_idx {
        return Some(clicked_tab_idx);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_ancestor_tab_has_a_muted_highlight() {
        let tab = TabInfo {
            active: true,
            ..Default::default()
        };
        let rendered = render_tab(
            "selected".into(),
            &tab,
            false,
            false,
            Styling::default(),
            "",
            &ModeInfo {
                session_dimmed: Some(true),
                ..Default::default()
            },
        );
        let background = PaletteColor::Rgb((215, 255, 127));
        let fill = Styling::default().text_unselected.background;
        let expected = ANSIStrings(&[
            style!(fill, background).paint(""),
            style!(PaletteColor::EightBit(16), background)
                .italic()
                .paint(" selected "),
            style!(background, fill).paint(""),
        ])
        .to_string();
        assert_eq!(rendered.part, expected);
    }

    fn render_for_mode(active: bool, mode_info: &ModeInfo) -> LinePart {
        render_tab(
            "tab".into(),
            &TabInfo {
                active,
                position: 2,
                ..Default::default()
            },
            false,
            false,
            mode_info.style.colors,
            "",
            mode_info,
        )
    }

    fn assert_ribbon_style(rendered: LinePart, declaration: StyleDeclaration, italic: bool) {
        let text_style = style!(declaration.base, declaration.background);
        let text_style = if italic {
            text_style.italic()
        } else {
            text_style.bold()
        };
        let fill = Styling::default().text_unselected.background;
        let expected = ANSIStrings(&[
            style!(fill, declaration.background).paint(""),
            text_style.paint(" tab "),
            style!(declaration.background, fill).paint(""),
        ])
        .to_string();
        assert_eq!(rendered.part, expected);
        assert_eq!(rendered.len, 5);
        assert_eq!(rendered.tab_index, Some(2));
    }

    #[test]
    fn unselected_ancestor_defaults_to_the_normal_unselected_ribbon() {
        let mode_info = ModeInfo {
            session_dimmed: Some(true),
            ..Default::default()
        };
        assert_ribbon_style(
            render_for_mode(false, &mode_info),
            mode_info.style.colors.ribbon_unselected,
            true,
        );
    }

    #[test]
    fn ancestor_overrides_only_apply_to_ancestors_of_the_focused_session() {
        let mut palette = Styling::default();
        palette.ribbon_ancestor_selected = Some(palette.text_selected);
        palette.ribbon_ancestor_unselected = Some(palette.frame_highlight);
        for (ascended, dimmed, is_ancestor) in [
            (None, Some(true), true),
            (None, None, false),
            (Some(true), None, false),
            (Some(true), Some(true), false),
        ] {
            let mut mode_info = ModeInfo {
                session_ascended: ascended,
                session_dimmed: dimmed,
                ..Default::default()
            };
            mode_info.style.colors = palette;
            for active in [false, true] {
                let expected = if is_ancestor {
                    if active {
                        palette.text_selected
                    } else {
                        palette.frame_highlight
                    }
                } else if ascended == Some(true) || !active {
                    palette.ribbon_unselected
                } else {
                    palette.ribbon_selected
                };
                assert_ribbon_style(
                    render_for_mode(active, &mode_info),
                    expected,
                    ascended == Some(true) || dimmed == Some(true),
                );
            }
        }
    }
}

pub(crate) fn get_clicked_line_part(
    tab_line: &[LinePart],
    mouse_click_col: usize,
) -> Option<&LinePart> {
    let mut len = 0;
    for tab_line_part in tab_line {
        if mouse_click_col >= len && mouse_click_col < len + tab_line_part.len {
            return Some(tab_line_part);
        }
        len += tab_line_part.len;
    }
    None
}
