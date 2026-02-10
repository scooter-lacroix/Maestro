use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::collections::HashMap;

use crate::conductor::model::{ConductorState, SubagentState};

pub fn render_subagent_tree(f: &mut Frame, area: Rect, state: &ConductorState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Subagents ")
        .border_style(Style::default().fg(Color::Yellow));

    let mut items = Vec::new();

    // Build parent-child mapping
    let mut root_agents = Vec::new();
    let mut children_map: HashMap<String, Vec<&SubagentState>> = HashMap::new();

    for agent in &state.subagents {
        if let Some(parent_id) = &agent.parent_id {
            if parent_id.is_empty() {
                root_agents.push(agent);
            } else {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(agent);
            }
        } else {
            root_agents.push(agent);
        }
    }

    // Sort root agents by started_at
    root_agents.sort_by_key(|a| a.started_at);

    for root in root_agents {
        render_agent_recursive(root, &children_map, 0, &mut items);
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            " No active subagents",
            Style::default().fg(Color::DarkGray),
        )])));
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_agent_recursive(
    agent: &SubagentState,
    children_map: &HashMap<String, Vec<&SubagentState>>,
    depth: usize,
    items: &mut Vec<ListItem<'static>>,
) {
    let indent = "  ".repeat(depth);
    let prefix = if depth == 0 { "● " } else { "└─ " };

    let style = match agent.status {
        crate::conductor::model::SubagentStatus::Running => Style::default().fg(Color::Green),
        crate::conductor::model::SubagentStatus::Completed => Style::default().fg(Color::DarkGray),
        crate::conductor::model::SubagentStatus::Error => Style::default().fg(Color::Red),
    };

    let content = Line::from(vec![
        Span::styled(indent, Style::default().fg(Color::DarkGray)),
        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
        Span::styled(agent.agent_type.clone(), style.add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(" ({:?})", agent.status),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);

    items.push(ListItem::new(content));

    if let Some(children) = children_map.get(&agent.id) {
        let mut sorted_children = (*children).clone();
        sorted_children.sort_by_key(|a| a.started_at);
        for child in sorted_children {
            render_agent_recursive(child, children_map, depth + 1, items);
        }
    }
}
