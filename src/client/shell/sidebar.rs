use super::*;
use ratatui::{
    text::Line,
    widgets::{Paragraph, Widget},
};

fn workspace_selection_background(palette: &Palette) -> ratatui::style::Color {
    if palette.selection_bg == ratatui::style::Color::Reset {
        palette.active_row_bg
    } else {
        palette.selection_bg
    }
}

pub(in crate::client::shell) fn workspace_active_background(
    palette: &Palette,
    navigating: bool,
) -> ratatui::style::Color {
    // The fallback cursor shares the active-row color; only fill the cursor while navigating.
    if navigating && palette.selection_bg == ratatui::style::Color::Reset {
        palette.sidebar_bg
    } else {
        palette.active_row_bg
    }
}

pub(in crate::client::shell) fn collapsed_sidebar_sections(
    area: Rect,
) -> (Rect, Option<u16>, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.is_empty() {
        return (Rect::default(), None, Rect::default());
    }
    if content.height < 7 {
        return (content, None, Rect::default());
    }
    let workspace_height = content.height.div_ceil(2);
    let divider_y = content.y + workspace_height;
    let detail_height = content.height.saturating_sub(workspace_height + 1);
    (
        Rect::new(content.x, content.y, content.width, workspace_height),
        Some(divider_y),
        Rect::new(content.x, divider_y + 1, content.width, detail_height),
    )
}

pub(crate) fn render_collapsed_sidebar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    selected_workspace_id: Option<&str>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let selection_background = workspace_selection_background(palette);
    let active_background = workspace_active_background(palette, selected_workspace_id.is_some());
    render_sidebar_background(buffer, area, palette);
    let (workspace_area, divider_y, detail_area) = collapsed_sidebar_sections(area);
    for (index, workspace) in snapshot
        .workspaces
        .iter()
        .take(workspace_area.height as usize)
        .enumerate()
    {
        let rect = Rect::new(
            workspace_area.x,
            workspace_area.y + index as u16,
            workspace_area.width,
            1,
        );
        let selected = selected_workspace_id == Some(workspace.workspace_id.as_str());
        if selected {
            buffer.set_style(rect, Style::default().bg(selection_background));
        } else if workspace.focused {
            buffer.set_style(rect, Style::default().bg(active_background));
        }
        let number_style = if selected {
            Style::default()
                .fg(palette.overlay1)
                .bg(selection_background)
        } else if workspace.focused {
            Style::default().fg(palette.text).bg(active_background)
        } else {
            Style::default().fg(palette.overlay0)
        };
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width.min(2),
            &format!("{:<2}", index + 1),
            number_style,
        );
        let status = workspace.agent_status;
        put_text(
            buffer,
            rect.x.saturating_add(2),
            rect.y,
            rect.width.saturating_sub(2),
            status_icon(status, config.status_indicators),
            Style::default().fg(status_color(status, palette)),
        );
        hits.workspaces.push(WorkspaceHit {
            rect,
            endpoint_id: ClientEndpointId::Local,
            workspace_id: workspace.workspace_id.clone(),
            indented: false,
            group_toggle: None,
        });
    }

    if let Some(divider_y) = divider_y {
        put_text(
            buffer,
            workspace_area.x,
            divider_y,
            workspace_area.width,
            &"─".repeat(workspace_area.width as usize),
            Style::default().fg(palette.surface_dim),
        );
    }

    let detail_content = Rect::new(
        detail_area.x,
        detail_area.y,
        detail_area.width,
        detail_area.height.saturating_sub(1),
    );
    for (index, pane_id) in super::ordered_agent_pane_ids(snapshot, config.agent_panel_sort)
        .into_iter()
        .take(detail_content.height as usize)
        .enumerate()
    {
        let Some(agent) = snapshot
            .agents
            .iter()
            .find(|agent| agent.pane_id == pane_id)
        else {
            continue;
        };
        let rect = Rect::new(
            detail_content.x,
            detail_content.y + index as u16,
            detail_content.width,
            1,
        );
        if agent.focused {
            buffer.set_style(rect, Style::default().bg(palette.active_row_bg));
        }
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width.min(2),
            &format!("{:<2}", index + 1),
            Style::default().fg(if agent.focused {
                palette.text
            } else {
                palette.overlay0
            }),
        );
        put_text(
            buffer,
            rect.x.saturating_add(2),
            rect.y,
            rect.width.saturating_sub(2),
            status_icon(agent.agent_status, config.status_indicators),
            Style::default().fg(status_color(agent.agent_status, palette)),
        );
        hits.agents.push((rect, pane_id));
    }
    hits.sidebar_toggle = if area.is_empty() || workspace_area.width == 0 {
        Rect::default()
    } else {
        Rect::new(
            workspace_area.x + workspace_area.width / 2,
            area.bottom().saturating_sub(1),
            1,
            1,
        )
    };
    put_text(
        buffer,
        hits.sidebar_toggle.x,
        hits.sidebar_toggle.y,
        hits.sidebar_toggle.width,
        "»",
        if super::super::global_menu::global_menu_attention(snapshot) {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.overlay0)
        },
    );
}

pub(crate) fn render_sidebar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    state: &mut ShellRenderState<'_>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    render_sidebar_background(buffer, area, palette);
    hits.sidebar_divider = if area.is_empty() {
        Rect::default()
    } else {
        Rect::new(area.right().saturating_sub(1), area.y, 1, area.height)
    };
    let workspace_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    put_text(
        buffer,
        workspace_area.x,
        workspace_area.y,
        workspace_area.width,
        " spaces",
        Style::default()
            .fg(palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );

    let rows = workspace_tree_rows(
        snapshot,
        &ClientEndpointId::Local,
        config,
        state.collapsed_groups,
        state.collapsed_agent_groups,
        super::agent_sidebar::agent_rows(snapshot, config, None),
    );
    let body = Rect::new(
        workspace_area.x,
        workspace_area.y.saturating_add(WORKSPACE_HEADER_ROWS),
        workspace_area.width,
        workspace_area
            .height
            .saturating_sub(WORKSPACE_HEADER_ROWS + 1),
    );
    hits.workspace_body = body;
    hits.agent_body = body;
    let row_heights = rows
        .iter()
        .map(WorkspaceTreeRow::height)
        .collect::<Vec<_>>();
    let gaps = tree_gaps(&rows);
    let mut metrics = super::scroll::list_scroll_metrics(
        &row_heights,
        &gaps,
        body.height,
        *state.workspace_scroll,
    );
    let reveal_navigation = !body.is_empty() && std::mem::take(state.reveal_navigation_workspace);
    let reveal_focus = !body.is_empty() && std::mem::take(state.reveal_focused_workspace);
    if reveal_navigation || reveal_focus {
        let focused_agent = (!reveal_navigation)
            .then(|| {
                rows.iter().position(
                    |row| matches!(row, WorkspaceTreeRow::Agent { agent, .. } if agent.focused),
                )
            })
            .flatten();
        if let Some(target) = focused_agent.or_else(|| {
            rows.iter().position(|row| {
                row.workspace_index().is_some_and(|index| {
                    let workspace = &snapshot.workspaces[index];
                    if reveal_navigation {
                        state.selected_workspace_id.is_some_and(|target| {
                            target.matches(&ClientEndpointId::Local, &workspace.workspace_id)
                        })
                    } else {
                        workspace.focused
                    }
                })
            })
        }) {
            *state.workspace_scroll = super::scroll::list_scroll_start_to_reveal(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
                target,
            );
            metrics = super::scroll::list_scroll_metrics(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
            );
        }
    }
    hits.workspace_max_scroll = metrics.max_offset_from_bottom;
    hits.workspace_scroll_metrics = Some(metrics);
    *state.workspace_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (index, row) in rows.iter().enumerate().skip(*state.workspace_scroll) {
        let height = row.height().min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, height);
        render_tree_row(
            buffer,
            rect,
            row,
            snapshot,
            &ClientEndpointId::Local,
            false,
            config,
            state,
            hits,
        );
        y = y.saturating_add(height + gaps[index]);
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.workspace_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, palette);
    }

    if let Some(row) = state.workspace_drop_indicator_row.filter(|row| {
        *row >= workspace_area.y.saturating_add(1)
            && *row < workspace_area.bottom().saturating_sub(1)
    }) {
        put_text(
            buffer,
            body.x,
            row,
            body.width,
            &"─".repeat(body.width as usize),
            Style::default().fg(palette.accent),
        );
    }

    let footer_y = workspace_area.bottom().saturating_sub(1);
    if config.mouse_capture {
        hits.new_workspace = Rect::new(
            workspace_area.x,
            footer_y,
            5.min(workspace_area.width),
            u16::from(workspace_area.height > 0),
        );
        put_text(
            buffer,
            workspace_area.x,
            footer_y,
            workspace_area.width,
            " new",
            Style::default().fg(palette.overlay0),
        );
        let attention = super::super::global_menu::global_menu_attention(snapshot);
        let launcher_width = if attention { 8 } else { 6 }.min(workspace_area.width);
        hits.global_launcher = Rect::new(
            workspace_area.right().saturating_sub(launcher_width),
            footer_y,
            launcher_width,
            1,
        );
        if attention {
            let start_x = workspace_area.right().saturating_sub(6);
            put_text(
                buffer,
                start_x,
                footer_y,
                2,
                "● ",
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            );
            put_text(
                buffer,
                start_x.saturating_add(2),
                footer_y,
                4,
                "menu",
                Style::default().fg(palette.overlay0),
            );
        } else {
            put_right_text(
                buffer,
                workspace_area,
                footer_y,
                "menu",
                Style::default().fg(palette.overlay0),
            );
        }
    }

    hits.sidebar_toggle = Rect::new(
        area.right().saturating_sub(2),
        area.bottom().saturating_sub(1),
        u16::from(area.width > 1),
        u16::from(area.height > 0),
    );
    put_text(
        buffer,
        hits.sidebar_toggle.x,
        hits.sidebar_toggle.y,
        hits.sidebar_toggle.width,
        "«",
        Style::default().fg(palette.overlay0),
    );
}

pub(in crate::client::shell) enum WorkspaceTreeRow {
    Workspace {
        entry: WorkspaceEntry,
        status: crate::api::schema::AgentStatus,
        tokens: Vec<Vec<crate::ui::ResolvedToken>>,
        has_agents: bool,
        collapsed: bool,
    },
    Agent {
        agent: super::agent_sidebar::AgentRow,
        entry: WorkspaceEntry,
        last: bool,
    },
}

impl WorkspaceTreeRow {
    pub(in crate::client::shell) fn workspace_index(&self) -> Option<usize> {
        match self {
            Self::Workspace { entry, .. } => Some(entry.index),
            Self::Agent { .. } => None,
        }
    }

    pub(in crate::client::shell) fn height(&self) -> u16 {
        let len = match self {
            Self::Workspace { tokens, .. } => tokens.len(),
            Self::Agent { agent, .. } => agent.rows.len(),
        };
        len.max(1).min(2) as u16
    }
}

pub(in crate::client::shell) fn tree_gaps(rows: &[WorkspaceTreeRow]) -> Vec<u16> {
    rows.iter()
        .enumerate()
        .map(|(index, _)| {
            u16::from(matches!(
                rows.get(index + 1),
                Some(WorkspaceTreeRow::Workspace { .. })
            ))
        })
        .collect()
}

pub(in crate::client::shell) fn normalize_tree_tokens(
    rows: &mut Vec<Vec<crate::ui::ResolvedToken>>,
) {
    let mut icon = None;
    rows.truncate(2);
    for row in rows.iter_mut() {
        while let Some(index) = row
            .iter()
            .position(|token| matches!(token.kind, crate::ui::ResolvedTokenKind::StateIcon))
        {
            let token = row.remove(index);
            if icon.is_none() {
                icon = Some(token);
            }
        }
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    rows[0].insert(
        0,
        icon.unwrap_or(crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }),
    );
}

pub(in crate::client::shell) fn workspace_tree_rows(
    snapshot: &ClientShellSnapshot,
    endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    collapsed_groups: &HashSet<String>,
    collapsed_agents: &HashSet<(ClientEndpointId, String)>,
    agents: Vec<super::agent_sidebar::AgentRow>,
) -> Vec<WorkspaceTreeRow> {
    let mut by_workspace: HashMap<String, Vec<super::agent_sidebar::AgentRow>> = HashMap::new();
    for agent in agents {
        by_workspace
            .entry(agent.workspace_id.clone())
            .or_default()
            .push(agent);
    }
    let mut rows = Vec::new();
    for entry in workspace_entries(snapshot, collapsed_groups) {
        let workspace = &snapshot.workspaces[entry.index];
        let status = displayed_workspace_status(snapshot, workspace, collapsed_groups);
        let agents = by_workspace
            .remove(&workspace.workspace_id)
            .unwrap_or_default();
        let has_agents = !agents.is_empty();
        let collapsed =
            collapsed_agents.contains(&(endpoint_id.clone(), workspace.workspace_id.clone()));
        let mut tokens = if collapsed {
            vec![vec![crate::ui::ResolvedToken {
                kind: crate::ui::ResolvedTokenKind::Workspace(workspace.label.clone()),
                style: Default::default(),
            }]]
        } else {
            workspace_rows(workspace, status, entry.indented, &config.spaces)
        };
        normalize_tree_tokens(&mut tokens);
        rows.push(WorkspaceTreeRow::Workspace {
            entry,
            status,
            tokens,
            has_agents,
            collapsed,
        });
        if collapsed {
            continue;
        }
        let count = agents.len();
        for (index, mut agent) in agents.into_iter().enumerate() {
            normalize_tree_tokens(&mut agent.rows);
            rows.push(WorkspaceTreeRow::Agent {
                agent,
                entry,
                last: index + 1 == count,
            });
        }
    }
    rows
}

pub(in crate::client::shell) fn render_tree_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &WorkspaceTreeRow,
    snapshot: &ClientShellSnapshot,
    endpoint_id: &ClientEndpointId,
    stale: bool,
    config: &ClientShellConfig,
    state: &ShellRenderState<'_>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let active = endpoint_id == state.active_endpoint_id;
    let hovered = state
        .agent_hover_point
        .is_some_and(|point| super::contains(rect, point));
    match row {
        WorkspaceTreeRow::Workspace {
            entry,
            status,
            tokens,
            has_agents,
            collapsed,
        } => {
            let workspace = &snapshot.workspaces[entry.index];
            let selected = state
                .selected_workspace_id
                .is_some_and(|target| target.matches(endpoint_id, &workspace.workspace_id));
            let workspace_focused = active && workspace.focused;
            let agent_focused = workspace_focused
                && snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.focused && agent.workspace_id == workspace.workspace_id);
            let focused = workspace_focused && !agent_focused;
            let guide_style = Style::default().fg(if workspace_focused {
                palette.subtext0
            } else {
                palette.overlay0
            });
            let dragged =
                active && state.dragged_workspace_id == Some(workspace.workspace_id.as_str());
            if hovered && !selected && !focused && !dragged {
                buffer.set_style(rect, Style::default().bg(palette.surface1));
            }
            render_workspace_rows(
                buffer,
                rect,
                *status,
                config.status_indicators,
                entry,
                tokens,
                focused,
                selected,
                state.selected_workspace_id.is_some(),
                dragged,
                *has_agents && !collapsed,
                *collapsed,
                guide_style,
                palette,
            );
            let empty = HashSet::new();
            let collapsed_groups = if endpoint_id.is_local() {
                state.collapsed_groups
            } else {
                state
                    .remote_collapsed_groups
                    .get(endpoint_id)
                    .unwrap_or(&empty)
            };
            let group_toggle = render_parent_group_toggle(
                buffer,
                rect,
                snapshot,
                entry.index,
                collapsed_groups,
                palette,
            );
            hits.workspaces.push(WorkspaceHit {
                rect,
                endpoint_id: endpoint_id.clone(),
                workspace_id: workspace.workspace_id.clone(),
                indented: entry.indented,
                group_toggle,
            });
            hits.agent_groups
                .push((rect, endpoint_id.clone(), workspace.workspace_id.clone()));
        }
        WorkspaceTreeRow::Agent { agent, entry, last } => {
            let workspace_id = &snapshot.workspaces[entry.index].workspace_id;
            if let Some((group_rect, _, _)) =
                hits.agent_groups
                    .last_mut()
                    .filter(|(_, group_endpoint, group_workspace)| {
                        group_endpoint == endpoint_id && group_workspace == workspace_id
                    })
            {
                group_rect.height = rect.bottom().saturating_sub(group_rect.y);
            } else {
                // A scrolled group can have visible agents without its header.
                hits.agent_groups
                    .push((rect, endpoint_id.clone(), workspace_id.clone()));
            }
            let focused = active && agent.focused;
            let guide_style =
                Style::default().fg(if active && snapshot.workspaces[entry.index].focused {
                    palette.subtext0
                } else {
                    palette.overlay0
                });
            if focused || hovered {
                buffer.set_style(
                    rect,
                    Style::default().bg(if focused {
                        palette.active_row_bg
                    } else {
                        palette.surface1
                    }),
                );
            }
            let indent = if entry.indented { 3 } else { 0 };
            let trunk_x = rect.x.saturating_add(indent);
            if !last && rect.height > 1 {
                put_text(
                    buffer,
                    trunk_x,
                    rect.y + 1,
                    rect.right().saturating_sub(trunk_x),
                    "│",
                    guide_style,
                );
            }
            if entry.indented && !entry.last_child {
                for y in rect.y..rect.bottom() {
                    put_text(
                        buffer,
                        rect.x,
                        y,
                        rect.width,
                        "│",
                        guide_style,
                    );
                }
            }
            let offset = rect.width.min(indent + 2);
            let content = Rect::new(rect.x + offset, rect.y, rect.width - offset, rect.height);
            super::agent_sidebar::render_agent_row(buffer, content, agent, focused, config);
            put_text(
                buffer,
                trunk_x,
                rect.y,
                rect.right().saturating_sub(trunk_x),
                if *last { "╰──" } else { "├──" },
                guide_style,
            );
            if endpoint_id.is_local() && state.endpoints.len() <= 1 {
                hits.agents.push((rect, agent.pane_id.clone()));
            } else {
                hits.endpoint_agents
                    .push((rect, endpoint_id.clone(), agent.pane_id.clone()));
            }
        }
    }
    if stale {
        buffer.set_style(
            rect,
            Style::default()
                .fg(palette.overlay0)
                .add_modifier(Modifier::DIM),
        );
    }
}

pub(crate) fn workspace_entries(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
) -> Vec<WorkspaceEntry> {
    let mut members = HashMap::<&str, Vec<usize>>::new();
    for (index, workspace) in snapshot.workspaces.iter().enumerate() {
        if let Some(worktree) = &workspace.worktree {
            members.entry(&worktree.key).or_default().push(index);
        }
    }
    let grouped = members
        .iter()
        .filter(|(_, indices)| {
            indices.len() >= 2
                && indices.iter().any(|index| {
                    snapshot.workspaces[*index]
                        .worktree
                        .as_ref()
                        .is_some_and(|worktree| !worktree.is_linked_worktree)
                })
        })
        .map(|(key, _)| *key)
        .collect::<HashSet<_>>();
    let mut emitted = HashSet::<&str>::new();
    let mut entries = Vec::new();
    for (index, workspace) in snapshot.workspaces.iter().enumerate() {
        let Some(worktree) = workspace
            .worktree
            .as_ref()
            .filter(|worktree| grouped.contains(worktree.key.as_str()))
        else {
            entries.push(WorkspaceEntry {
                index,
                indented: false,
                last_child: false,
            });
            continue;
        };
        if !emitted.insert(&worktree.key) {
            continue;
        }
        let Some(group_members) = members.get(worktree.key.as_str()) else {
            continue;
        };
        let parent = group_members
            .iter()
            .copied()
            .find(|member| {
                snapshot.workspaces[*member]
                    .worktree
                    .as_ref()
                    .is_some_and(|worktree| !worktree.is_linked_worktree)
            })
            .unwrap_or(index);
        entries.push(WorkspaceEntry {
            index: parent,
            indented: false,
            last_child: false,
        });
        if collapsed_groups.contains(&worktree.key) {
            if let Some(active) = group_members
                .iter()
                .copied()
                .find(|member| *member != parent && snapshot.workspaces[*member].focused)
            {
                entries.push(WorkspaceEntry {
                    index: active,
                    indented: true,
                    last_child: true,
                });
            }
            continue;
        }
        let children = group_members
            .iter()
            .copied()
            .filter(|member| *member != parent)
            .collect::<Vec<_>>();
        for (child_index, child) in children.iter().enumerate() {
            entries.push(WorkspaceEntry {
                index: *child,
                indented: true,
                last_child: child_index + 1 == children.len(),
            });
        }
    }
    entries
}

fn parent_group_key(snapshot: &ClientShellSnapshot, index: usize) -> Option<String> {
    let workspace = snapshot.workspaces.get(index)?;
    let worktree = workspace.worktree.as_ref()?;
    if worktree.is_linked_worktree {
        return None;
    }
    (snapshot
        .workspaces
        .iter()
        .filter(|candidate| {
            candidate
                .worktree
                .as_ref()
                .is_some_and(|candidate| candidate.key == worktree.key)
        })
        .count()
        >= 2)
        .then(|| worktree.key.clone())
}

pub(in crate::client::shell) fn render_parent_group_toggle(
    buffer: &mut Buffer,
    workspace_rect: Rect,
    snapshot: &ClientShellSnapshot,
    workspace_index: usize,
    collapsed_groups: &HashSet<String>,
    palette: &Palette,
) -> Option<(Rect, String)> {
    if workspace_rect.is_empty() {
        return None;
    }
    let key = parent_group_key(snapshot, workspace_index)?;
    let toggle = Rect::new(
        workspace_rect.right().saturating_sub(1),
        workspace_rect.y,
        1,
        1,
    );
    put_text(
        buffer,
        toggle.x,
        toggle.y,
        toggle.width,
        if collapsed_groups.contains(&key) {
            "▸"
        } else {
            "▾"
        },
        Style::default().fg(palette.accent),
    );
    Some((toggle, key))
}

pub(in crate::client::shell) fn displayed_workspace_status(
    snapshot: &ClientShellSnapshot,
    workspace: &ClientShellWorkspace,
    collapsed_groups: &HashSet<String>,
) -> crate::api::schema::AgentStatus {
    let Some(worktree) = workspace
        .worktree
        .as_ref()
        .filter(|worktree| !worktree.is_linked_worktree)
    else {
        return workspace.agent_status;
    };
    if !collapsed_groups.contains(&worktree.key) {
        return workspace.agent_status;
    }
    snapshot
        .workspaces
        .iter()
        .filter(|candidate| {
            candidate
                .worktree
                .as_ref()
                .is_some_and(|candidate| candidate.key == worktree.key)
        })
        .map(|candidate| candidate.agent_status)
        .max_by_key(|status| status_priority(*status))
        .unwrap_or(workspace.agent_status)
}

pub(in crate::client::shell) fn workspace_rows(
    workspace: &ClientShellWorkspace,
    status: crate::api::schema::AgentStatus,
    indented: bool,
    config: &SpacesSidebarConfig,
) -> Vec<Vec<crate::ui::ResolvedToken>> {
    let label = if indented && !workspace.custom_label {
        workspace
            .branch
            .as_deref()
            .and_then(|branch| branch.strip_prefix("worktree/").or(Some(branch)))
            .unwrap_or(&workspace.label)
    } else {
        &workspace.label
    };
    let token_values = workspace.tokens.iter().cloned().collect::<HashMap<_, _>>();
    crate::ui::sidebar_space_rows(
        config,
        crate::ui::SpaceTokenContext {
            workspace: label,
            branch: workspace.branch.as_deref(),
            state_text: status_text(status),
            ahead_behind: workspace.git_ahead_behind,
            tokens: &token_values,
            suppress_git_details: indented,
        },
    )
}

pub(in crate::client::shell) fn render_workspace_rows(
    buffer: &mut Buffer,
    area: Rect,
    status: crate::api::schema::AgentStatus,
    indicators: crate::config::StatusIndicatorStyle,
    entry: &WorkspaceEntry,
    rows: &[Vec<crate::ui::ResolvedToken>],
    focused: bool,
    selected: bool,
    navigating: bool,
    dragged: bool,
    has_children: bool,
    folded: bool,
    guide_style: Style,
    palette: &Palette,
) {
    for (row_index, row) in rows.iter().enumerate() {
        let y = area.y + row_index as u16;
        if y >= area.bottom() {
            break;
        }
        let indent = if entry.indented { 3 } else { 0 };
        if entry.indented {
            let prefix = if row_index == 0 {
                if entry.last_child {
                    "╰──"
                } else {
                    "├──"
                }
            } else if entry.last_child {
                "   "
            } else {
                "│  "
            };
            put_text(buffer, area.x, y, area.width, prefix, guide_style);
        }
        if row_index > 0 && has_children {
            let trunk_x = area.x.saturating_add(indent);
            put_text(
                buffer,
                trunk_x,
                y,
                area.right().saturating_sub(trunk_x),
                "│",
                guide_style,
            );
        }
        let x = area
            .x
            .saturating_add(indent + if row_index == 0 { 0 } else { 2 })
            .min(area.right());
        let width = area
            .right()
            .saturating_sub(if folded { 0 } else { 2 })
            .saturating_sub(x);
        let text_width = width.saturating_sub(if folded { 2 } else { 0 });
        let workspace_style = Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD);
        let secondary_style = Style::default().fg(if focused {
            palette.mauve
        } else {
            palette.overlay0
        });
        let spans = crate::ui::resolved_token_spans(
            row,
            (
                status_icon(status, indicators),
                Style::default().fg(status_color(status, palette)),
            ),
            Style::default().fg(status_color(status, palette)),
            workspace_style,
            secondary_style,
            Style::default().fg(palette.overlay1),
            palette,
            text_width as usize,
        );
        let line = Line::from(spans);
        let name_width = line.width().min(text_width as usize) as u16;
        Paragraph::new(line).render(Rect::new(x, y, text_width, 1), buffer);
        if folded {
            let dots_x = x + name_width + u16::from(name_width > 0);
            for dot_x in dots_x..x + width {
                buffer[(dot_x, y)].set_symbol("·").set_fg(palette.overlay0);
            }
        }
    }

    let background = if selected {
        Some(workspace_selection_background(palette))
    } else if dragged {
        Some(palette.surface1)
    } else if focused {
        Some(workspace_active_background(palette, navigating))
    } else {
        None
    };
    if let Some(background) = background {
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                buffer[(x, y)].set_bg(background);
            }
        }
    }
}
