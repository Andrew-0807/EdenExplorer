# Settings, Obsidian style

Goal: one modal, left list of sections under a muted group header, right pane of flat rows. Theme editor folds into Appearance. Storage and semantics unchanged.

## Sizing
`body_size(palette) = palette.text_size`. Row description `text_size - 2`, section title `text_size + 6`, group header `text_size - 2`. No fixed `+4`.

## Layout
- Sidebar: search box, group header "Options", then sections. Selected = subtle filled highlight (no accent bar).
- Pane: section title, flat rows split by 1px dividers. No cards.
- Row: name + muted one-line description left, control right. Toggle / dropdown / slider with value.
- Each section may end with a collapsible "Advanced".
- Search (`search_admits`, Ctrl+F, hidden empty groups) unchanged. While searching, every section is listed.

## Sections (SettingsTab)
1. General: language, start folder, window size (+ width/height), remember open tabs, date style, 24h time. Advanced: reset settings, reset favorites.
2. Files: show hidden, folder scanning, file icons, extra action buttons, default columns shown. Advanced: default column widths, clear folder cache.
3. Interface: item checkboxes, toolbar layout, status bar visible, "Explorer-like" preset. Advanced: toolbar items, status bar items, sidebar sections.
4. Appearance (was Appearance + theme editor): base theme (Light/Dark/System), preset, accent color (+ follow Windows accent), font, font size slider, interface zoom slider, backdrop, transparency slider. Advanced: density, sizes and shapes, selection, icon style, "Customize colors" grouped Window / Sidebar / Item list / Tabs / Menus and popups / Text, export / import / reset.
5. Hotkeys: `hotkey_editor::draw`.
6. Integrations: Windows context menu, zoxide auto-add.

About stays its own window.

## Theme editor
Separate window removed. Topbar "customize theme" opens Settings > Appearance. Settings holds `&mut ThemeCustomizer` and queues a `ThemeCustomizerAction` in `ThemeCustomizer::pending_action`; the existing handler applies, saves, exports and imports. It edits the active mode's palette. Light/Dark/System writes `theme_follow_system` and queues `requested_mode`.
