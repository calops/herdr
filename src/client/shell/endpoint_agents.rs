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

pub(super) fn render_expanded(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    collapsed_agent_groups: &HashSet<(ClientEndpointId, String)>,
    hover_point: Option<(u16, u16)>,
    hits: &mut ShellHitMap,
) {
    if !super::agent_sidebar::render_agent_panel_header(
        buffer,
        area,
        agent_view_label,
        config,
        hits,
    ) {
        return;
    }
    let (grouped, rows) = expanded_agent_rows(
        endpoints,
        active_endpoint_id,
        config,
        collapsed_agent_groups,
    );
    super::agent_sidebar::render_agent_list(
        buffer,
        area,
        &rows,
        agent_view_label.map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        |row| match row {
            super::agent_sidebar::AgentListRow::Group(_) => 1,
            super::agent_sidebar::AgentListRow::Agent(row) => row.agent.rows.len(),
        },
        |buffer, rect, row, hits| match row {
            super::agent_sidebar::AgentListRow::Group(group) => {
                super::agent_sidebar::render_agent_group(
                    buffer,
                    rect,
                    group,
                    config,
                    hover_point,
                    hits,
                );
            }
            super::agent_sidebar::AgentListRow::Agent(row) => {
                if grouped && row.agent.focused {
                    buffer.set_style(rect, Style::default().bg(config.palette.active_row_bg));
                } else if !row.agent.focused
                    && hover_point.is_some_and(|point| super::contains(rect, point))
                {
                    buffer.set_style(rect, Style::default().bg(config.palette.surface1));
                }
                let content = if grouped {
                    super::agent_sidebar::indented_agent_rect(rect)
                } else {
                    rect
                };
                super::agent_sidebar::render_agent_row(buffer, content, &row.agent, config);
                if row.stale {
                    buffer.set_style(
                        rect,
                        Style::default()
                            .fg(config.palette.overlay0)
                            .add_modifier(Modifier::DIM),
                    );
                }
                hits.endpoint_agents.push((
                    rect,
                    row.endpoint_id.clone(),
                    row.agent.pane_id.clone(),
                ));
            }
        },
    );
}

impl ClientShellState {
    pub(super) fn reveal_endpoint_agent(
        &mut self,
        endpoint_id: &ClientEndpointId,
        pane_id: &str,
        body_height: u16,
    ) {
        if let Some(workspace_id) = self
            .endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
            .and_then(|snapshot| {
                snapshot
                    .agents
                    .iter()
                    .find(|agent| agent.pane_id == pane_id)
            })
            .map(|agent| agent.workspace_id.clone())
        {
            self.collapsed_agent_groups
                .remove(&(endpoint_id.clone(), workspace_id));
        }
        if body_height == 0 {
            return;
        }
        let (_, rows) = expanded_agent_rows(
            &self.endpoints,
            &self.active_endpoint_id,
            &self.config,
            &self.collapsed_agent_groups,
        );
        let Some(target) = rows.iter().position(|row| {
            matches!(row, super::agent_sidebar::AgentListRow::Agent(row)
                if &row.endpoint_id == endpoint_id && row.agent.pane_id == pane_id)
        }) else {
            return;
        };
        let heights = rows
            .iter()
            .map(|row| match row {
                super::agent_sidebar::AgentListRow::Group(_) => 1,
                super::agent_sidebar::AgentListRow::Agent(row) => {
                    row.agent.rows.len().max(1).min(u16::MAX as usize) as u16
                }
            })
            .collect::<Vec<_>>();
        let mut gaps = vec![self.config.agents.row_gap; rows.len()];
        if let Some(last) = gaps.last_mut() {
            *last = 0;
        }
        self.agent_scroll = super::scroll::list_scroll_start_to_reveal(
            &heights,
            &gaps,
            body_height,
            self.agent_scroll,
            target,
        );
    }
}

struct EndpointAgentRow {
    endpoint_id: ClientEndpointId,
    machine_label: String,
    stale: bool,
    agent: super::agent_sidebar::AgentRow,
}

fn expanded_agent_rows(
    endpoints: &[ClientShellEndpoint],
    active_endpoint_id: &ClientEndpointId,
    config: &ClientShellConfig,
    collapsed_agent_groups: &HashSet<(ClientEndpointId, String)>,
) -> (
    bool,
    Vec<super::agent_sidebar::AgentListRow<EndpointAgentRow>>,
) {
    use super::agent_sidebar::{AgentGroupRow, AgentListRow};
    let grouped = config.agent_panel_sort == crate::config::AgentPanelSortConfig::Spaces
        && !endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == active_endpoint_id)
            .is_some_and(|endpoint| {
                endpoint
                    .snapshot
                    .as_deref()
                    .is_some_and(|snapshot| snapshot.agent_view_label.is_some())
                    || matches!(
                        ClientShellState::endpoint_agent_view(endpoint),
                        Some(Ok(Some(_)))
                    )
            });
    let rows = agent_rows(endpoints, active_endpoint_id, config);
    if !grouped {
        return (false, rows.into_iter().map(AgentListRow::Agent).collect());
    }
    let rows = super::agent_sidebar::group_agent_rows(
        rows,
        |row| (&row.endpoint_id, row.agent.workspace_id.as_str()),
        |row| {
            let endpoint = endpoints
                .iter()
                .find(|endpoint| endpoint.endpoint_id == row.endpoint_id)
                .expect("agent row has an endpoint");
            let workspace = endpoint
                .snapshot
                .as_deref()
                .and_then(|snapshot| {
                    snapshot
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.workspace_id == row.agent.workspace_id)
                })
                .expect("agent row has a workspace");
            AgentGroupRow {
                endpoint_id: row.endpoint_id.clone(),
                workspace_id: workspace.workspace_id.clone(),
                label: format!("{} · {}", endpoint.label, workspace.label),
                status: workspace.agent_status,
                collapsed: collapsed_agent_groups
                    .contains(&(row.endpoint_id.clone(), workspace.workspace_id.clone())),
                stale: row.stale,
            }
        },
    );
    (true, rows)
}

fn agent_rows(
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
