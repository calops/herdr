use super::render::put_text;
use super::*;

pub(super) fn render_collapsed(
    buffer: &mut Buffer,
    area: Rect,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    let rows = agent_rows(endpoints, active_endpoint_id, config);
    for (index, row) in rows.into_iter().take(area.height as usize).enumerate() {
        let rect = Rect::new(area.x, area.y + index as u16, area.width, 1);
        if row.agent.focused {
            buffer.set_style(rect, Style::default().bg(config.palette.active_row_bg));
        }
        let initial = row.machine_label.chars().next().unwrap_or('?');
        put_text(
            buffer,
            rect.x,
            rect.y,
            rect.width,
            &format!(
                "{initial}{}",
                status_icon(row.agent.status, config.status_indicators)
            ),
            Style::default()
                .fg(if row.stale {
                    config.palette.overlay0
                } else {
                    status_color(row.agent.status, &config.palette)
                })
                .add_modifier(if row.stale {
                    Modifier::DIM
                } else {
                    Modifier::empty()
                }),
        );
        hits.endpoint_agents
            .push((rect, row.endpoint_id, row.agent.pane_id));
    }
}

impl ClientShellState {
    pub(super) fn reveal_endpoint_agent(
        &mut self,
        endpoint_id: &ClientEndpointId,
        pane_id: &str,
        body_height: u16,
    ) {
        let Some(workspace) = self
            .endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
            .and_then(|snapshot| {
                let agent = snapshot
                    .agents
                    .iter()
                    .find(|agent| agent.pane_id == pane_id)?;
                snapshot
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.workspace_id == agent.workspace_id)
            })
        else {
            return;
        };
        let workspace_id = workspace.workspace_id.clone();
        let worktree_key = workspace
            .worktree
            .as_ref()
            .map(|worktree| worktree.key.clone());
        self.collapsed_endpoints.remove(endpoint_id);
        self.collapsed_agent_groups
            .remove(&(endpoint_id.clone(), workspace_id));
        if let Some(key) = worktree_key {
            if endpoint_id.is_local() {
                self.collapsed_groups.remove(&key);
            } else if let Some(groups) = self.remote_collapsed_groups.get_mut(endpoint_id) {
                groups.remove(&key);
            }
        }
        self.reveal_focused_workspace = false;
        self.reveal_navigation_workspace = false;
        if body_height == 0 {
            return;
        }
        if self.endpoints.len() <= 1 && endpoint_id.is_local() {
            let Some(snapshot) = self
                .endpoints
                .iter()
                .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
                .and_then(|endpoint| endpoint.snapshot.as_deref())
            else {
                return;
            };
            let rows = super::sidebar::workspace_tree_rows(
                snapshot,
                endpoint_id,
                &self.config,
                &self.collapsed_groups,
                &self.collapsed_agent_groups,
                super::agent_sidebar::agent_rows(snapshot, &self.config, None),
            );
            let Some(target) = rows.iter().position(|row| {
                matches!(row, super::sidebar::WorkspaceTreeRow::Agent { agent, .. }
                    if agent.pane_id == pane_id)
            }) else {
                return;
            };
            let heights = rows
                .iter()
                .map(super::sidebar::WorkspaceTreeRow::height)
                .collect::<Vec<_>>();
            let gaps = super::sidebar::tree_gaps(&rows);
            self.workspace_scroll = super::scroll::list_scroll_start_to_reveal(
                &heights,
                &gaps,
                body_height,
                self.workspace_scroll,
                target,
            );
            return;
        }
        let rows = super::endpoint_sidebar::expanded_tree_rows(
            &self.endpoints,
            &self.active_endpoint_id,
            &self.config,
            &self.collapsed_endpoints,
            &self.collapsed_groups,
            &self.remote_collapsed_groups,
            &self.collapsed_agent_groups,
        );
        let Some(target) = rows.iter().position(|row| {
            matches!(row, super::endpoint_sidebar::EndpointTreeRow::Item {
                endpoint,
                row: super::sidebar::WorkspaceTreeRow::Agent { agent, .. },
            } if &self.endpoints[*endpoint].endpoint_id == endpoint_id && agent.pane_id == pane_id)
        }) else {
            return;
        };
        let heights = rows
            .iter()
            .map(super::endpoint_sidebar::EndpointTreeRow::height)
            .collect::<Vec<_>>();
        let gaps = super::endpoint_sidebar::expanded_tree_gaps(&rows);
        self.workspace_scroll = super::scroll::list_scroll_start_to_reveal(
            &heights,
            &gaps,
            body_height,
            self.workspace_scroll,
            target,
        );
    }
}

pub(super) struct EndpointAgentRow {
    pub(super) endpoint_id: ClientEndpointId,
    pub(super) machine_label: String,
    pub(super) stale: bool,
    pub(super) agent: super::agent_sidebar::AgentRow,
}

pub(super) fn agent_rows(
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
) -> Vec<EndpointAgentRow> {
    let mut rendered_rows = endpoints
        .iter()
        .filter_map(|endpoint| {
            endpoint.snapshot.as_deref().map(|snapshot| {
                snapshot
                    .agents
                    .iter()
                    .filter_map(|agent| {
                        super::agent_sidebar::agent_row(
                            snapshot,
                            &agent.pane_id,
                            config,
                            Some(&endpoint.label),
                        )
                    })
                    .map(|agent| ((endpoint.endpoint_id.clone(), agent.pane_id.clone()), agent))
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect::<HashMap<_, _>>();

    super::aggregate_navigation::aggregate_agent_rows(
        endpoints,
        active_endpoint_id,
        config.agent_panel_sort,
    )
    .into_iter()
    .filter_map(|row| {
        let key = (row.endpoint.endpoint_id.clone(), row.agent.pane_id.clone());
        let mut agent = rendered_rows.remove(&key)?;
        agent.focused &= row.endpoint.endpoint_id == active_endpoint_id;
        Some(EndpointAgentRow {
            endpoint_id: row.endpoint.endpoint_id.clone(),
            machine_label: row.endpoint.label.to_owned(),
            stale: row.endpoint.stale(),
            agent,
        })
    })
    .collect()
}
