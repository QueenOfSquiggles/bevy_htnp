use bevy::{
    app::App,
    ecs::component::{ComponentHooks, StorageType},
    prelude::{Component, Query},
};
use goals::{Goal, GoalEvaluation};
use providers::{GoalProvider, TaskProvider};

use crate::{
    data::{Requirements, WorldState},
    events::observer_handle_invalidated_plan,
    tasks::Task,
};

pub mod goals;
pub mod plan_data;
pub mod providers;
pub mod tree;

pub(crate) fn plugin(app: &mut App) {
    providers::plugin(app);
}

#[derive(Default)]
pub struct HtnAgent {
    pub goals: Vec<Goal>,
    pub current_plan: Option<plan_data::Plan>,
    pub available_tasks: Vec<Task>,
    pub goal_eval: GoalEvaluation,
}

#[derive(Component, Default, Clone, Debug)]
pub struct HtnAgentPlanningPriority(pub f32);

impl HtnAgent {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_task(&mut self, task: Task) -> &mut Self {
        self.available_tasks.push(task);
        self
    }

    pub fn add_goal(
        &mut self,
        name: impl Into<String>,
        goal: impl Into<Requirements>,
        static_utility: f32,
    ) -> &mut Self {
        self.goals.push(Goal::new(name, goal, static_utility));
        self
    }
    pub fn has_plan(&self) -> bool {
        self.current_plan.is_some()
    }

    pub fn get_next_goal(&self, world: &WorldState) -> Option<Goal> {
        self.goal_eval.next_goal(&self.goals, world)
    }
}

pub fn system_collect_agent_tasks_from_providers(
    mut query: Query<(&dyn TaskProvider, &mut HtnAgent)>,
) {
    for (providers, mut agent) in query.iter_mut() {
        let mut tasks = Vec::<Task>::new();
        for p in providers {
            tasks.append(&mut p.tasks());
        }
        agent.available_tasks = tasks;
    }
}

pub fn system_collect_agent_goals_from_providers(
    mut query: Query<(&dyn GoalProvider, &mut HtnAgent)>,
) {
    for (providers, mut agent) in query.iter_mut() {
        let mut goals = Vec::<Goal>::new();
        for p in providers {
            goals.append(&mut p.goals());
        }
        agent.goals = goals;
    }
}

impl Component for HtnAgent {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        // when an HTN Agent is created, start observing for a plan invalidated event.
        // This allows any system to easily mark the current plan as invalid without excessive dependencies

        hooks.on_add(|mut world, entity, _| {
            world
                .commands()
                .entity(entity)
                .observe(observer_handle_invalidated_plan);
        });
    }
}

#[cfg(test)]
mod tests {

    use crate::data::Requirements;

    use super::*;

    #[test]
    fn goal_picking_planning() {
        let mut agent = HtnAgent::default();
        agent.goal_eval = GoalEvaluation::Top;
        let goal_a: WorldState = vec![("A", true.into())].into_iter().into();
        let goal_b: WorldState = vec![("B", true.into())].into_iter().into();
        let goal_c: WorldState = vec![("C", true.into())].into_iter().into();

        let world_ab = WorldState::new().add("A", true).add("B", true).build();
        let world_b = WorldState::new().add("B", true).build();
        let world_not_a = WorldState::new().add("A", false).add("B", true).build();

        agent.add_goal("A", goal_a.clone(), 1.0);
        agent.add_goal("B", goal_b.clone(), 1.0);
        agent.add_goal("C", goal_c.clone(), 1.0);

        let next_goal = agent.get_next_goal(&WorldState::new());
        assert!(next_goal.is_some());
        let next_goal = next_goal.unwrap();

        assert!(next_goal.requires.validate(&world_ab));
        assert!(!next_goal.requires.validate(&world_b));
        assert!(!next_goal.requires.validate(&world_not_a));
    }

    #[test]
    fn requirements_validation() {
        let req = Requirements::new()
            .req_equals("bool_eq", true)
            .req_equals("str_eq", "something")
            .req_equals("num_eq", 3.1415)
            .req_has("any_key")
            .req_greater("num_grt", 0.0)
            .req_less("num_lst", 0.0)
            .build();

        let valid_world = WorldState::new()
            .add("bool_eq", true)
            .add("str_eq", "something")
            .add("num_eq", 3.1415)
            .add("any_key", 25.)
            .add("num_grt", 10.)
            .add("num_lst", -12.36)
            .build();

        let invalid_world = WorldState::new()
            .add("bool_eq", false)
            .add("str_eq", "else")
            .add("num_eq", 3.)
            .add("num_grt", -10.)
            .add("num_lst", 12.36)
            .build();

        assert!(req.validate(&valid_world));
        assert!(!req.validate(&WorldState::new()));
        assert!(!req.validate(&invalid_world));
    }
}
