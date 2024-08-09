use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::{
    data::{HtnSettings, WorldState},
    planning::HtnAgent,
    prelude::{plan_data::TimeSlicedTreeGen, HtnAgentPlanningPriority},
    tasks::TaskRegistry,
};

#[derive(Component)]
pub struct HtnAgentWorld(pub WorldState);

#[derive(Component, Debug)]
pub struct HtnAgentPlan {
    pub plan_stack: Vec<String>,
}

#[derive(Component)]
pub struct HtnAgentCurrentTask(pub String);

#[derive(Component, PartialEq)]
pub enum HtnAgentState {
    // TODO: should this be constructed in a way that allows observers?
    Running,
    Success,
    Failure,
}

#[allow(clippy::type_complexity)]
pub fn system_extract_plans_for_unplanned_agents(
    query: Query<
        (
            Entity,
            &HtnAgent,
            &TimeSlicedTreeGen,
            Option<&HtnAgentWorld>,
            Option<&HtnAgentPlanningPriority>,
        ),
        Without<HtnAgentPlan>,
    >,
    world: Res<WorldState>,
    settings: Res<HtnSettings>,
    mut command: Commands,
) {
    let mut vec: Vec<(
        Entity,
        &HtnAgent,
        &TimeSlicedTreeGen,
        Option<&HtnAgentWorld>,
        Option<&HtnAgentPlanningPriority>,
    )> = query.iter().collect();

    if !settings.disable_priority_sort.unwrap_or_default() {
        // TODO: someday this should be replaced by bevy's table sorting feature that is in development as of writing
        vec.sort_by(|a, b| {
            a.4.cloned()
                .unwrap_or_default()
                .0
                .total_cmp(&b.4.cloned().unwrap_or_default().0)
        });
    }
    for (entity, agent, tree, ctx, _) in vec {
        let mut agent_context = world.clone();
        if let Some(w) = ctx {
            agent_context.append(&w.0);
        }
        let Some(goal) = agent.get_next_goal(&agent_context) else {
            continue;
        };

        let Some(plan) = tree.plans.get(&goal.name) else {
            continue;
        };
        command.entity(entity).insert(HtnAgentPlan {
            plan_stack: plan.decompose_tasks(),
        });
    }
}

pub fn system_handle_agent_state_changes(
    mut query: Query<(
        Entity,
        &mut HtnAgentPlan,
        Option<&HtnAgentState>,
        Option<&HtnAgentCurrentTask>,
    )>,
    task_registry: Res<TaskRegistry>,
    mut command: Commands,
) {
    for (entity, mut plan, state, task) in query.iter_mut() {
        if let Some(agent_state) = state {
            match agent_state {
                // running states process as handled by that task ( user defined system(s) )
                HtnAgentState::Running => continue,
                // when a task succeeds, push this state. Old task removed and next task injected
                HtnAgentState::Success => {
                    if let Some(next_task) = plan.plan_stack.pop() {
                        if let Some(prev_task) = task {
                            try_remove_previous_task(
                                &mut command.entity(entity),
                                &task_registry,
                                prev_task,
                            );
                        }
                        push_task_to_agent(next_task, &mut command.entity(entity), &task_registry);
                    } else {
                        command
                            .entity(entity)
                            .remove::<(HtnAgentCurrentTask, HtnAgentState, HtnAgentPlan)>();
                    }
                }
                // When a task fails for some reason we push this state, which purges existing execution data
                HtnAgentState::Failure => {
                    command
                        .entity(entity)
                        .remove::<(HtnAgentCurrentTask, HtnAgentState, HtnAgentPlan)>();
                }
            }
        } else if let Some(next_task) = plan.plan_stack.pop() {
            push_task_to_agent(next_task, &mut command.entity(entity), &task_registry);
        } else {
            command
                .entity(entity)
                .remove::<(HtnAgentCurrentTask, HtnAgentState, HtnAgentPlan)>();
            warn!("Failed to initialize a plan for entity {}", entity);
        }
    }
}

fn push_task_to_agent(
    task: String,
    entity: &mut EntityCommands,
    task_registry: &Res<TaskRegistry>,
) {
    let Some(task_data) = task_registry.get_named(&task) else {
        return;
    };
    task_data.add(entity);
    entity.insert((HtnAgentCurrentTask(task), HtnAgentState::Running));
}

fn try_remove_previous_task(
    entity: &mut EntityCommands,
    task_registry: &Res<TaskRegistry>,
    previous: &HtnAgentCurrentTask,
) {
    let Some(task) = task_registry.get_named(&previous.0) else {
        return;
    };
    task.remove(entity);
}
