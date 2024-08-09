use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    time::{Duration, Instant},
    u32,
};

use bevy::{
    log::error,
    prelude::{Component, Query, Res, With},
};

use crate::{
    data::{HtnSettings, WorldState},
    prelude::HtnAgentWorld,
    tasks::{Task, TaskRegistry},
};

use std::collections::VecDeque;

use super::{goals::Goal, tree::Node, HtnAgent};

#[derive(Default, Clone)]
pub struct Plan {
    pub tasks: VecDeque<Task>,
    pub cost: f32,
}

impl Plan {
    pub fn decompose_tasks(&self) -> Vec<String> {
        Task::decompose_iter(self.tasks.clone().into_iter())
    }

    pub fn simple_print_tasks(&self) -> Vec<String> {
        self.tasks.iter().map(|t| t.name()).collect()
    }
}

impl Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Plan")
            .field(&format_args!("Steps: {}", self.tasks.len()))
            .finish()
    }
}

#[derive(Component)]
pub struct TimeSlicedTreeGen {
    pub active_nodes: VecDeque<Arc<Node<PlanNode>>>,
    pub valid_nodes: Vec<Arc<Node<PlanNode>>>,
    pub goals: Vec<Goal>,
    pub plans: HashMap<String, Plan>,
    pub available_tasks: Vec<Task>,
}

#[derive(Debug, Clone)]
pub struct PlanNode {
    pub task: Option<Task>,
    pub world: WorldState,
    pub cost: f32,
    pub depth: u32,
}

impl TimeSlicedTreeGen {
    pub fn new() -> Self {
        Self {
            active_nodes: VecDeque::new(),
            valid_nodes: Vec::new(),
            goals: Vec::new(),
            plans: HashMap::new(),
            available_tasks: Vec::new(),
        }
    }

    pub fn new_initialized(tasks: Vec<Task>, goals: Vec<Goal>) -> Self {
        let mut sorted_goals = goals;
        sorted_goals.sort_by(|a, b| a.utility.total_cmp(&b.utility));
        Self {
            active_nodes: VecDeque::new(),
            valid_nodes: Vec::new(),
            goals: sorted_goals,
            plans: HashMap::new(),
            available_tasks: tasks,
        }
    }

    pub fn generate_for_duration(
        &mut self,
        registry: &TaskRegistry,
        current_world: &WorldState,
        duration: Option<Duration>,
        max_node_depth: Option<u32>,
    ) {
        let Some(goal) = self.goals.last().cloned() else {
            return;
        };
        let timer = Instant::now();
        self.try_seed_active_nodes(registry, current_world);

        loop {
            self.generate_single(&goal, registry, max_node_depth);
            self.try_emit_single(&goal);

            if let Some(duration) = duration {
                if timer.elapsed() >= duration {
                    break;
                }
            }
            if self.active_nodes.is_empty() {
                break;
            }
        }
    }

    /// rather that limiting generation for a specific time frame, hold the thread until processing is completed. This isn't great on performance, but does create results and is good for testing
    pub fn generate_to_completion(
        &mut self,
        registry: &TaskRegistry,
        current_world: &WorldState,
        max_node_depth: Option<u32>,
    ) {
        let Some(goal) = self.goals.last().cloned() else {
            return;
        };
        self.try_seed_active_nodes(registry, current_world);

        loop {
            self.generate_single(&goal, registry, max_node_depth);
            self.try_emit_single(&goal);

            if self.active_nodes.is_empty() {
                break;
            }
        }
    }

    fn try_seed_active_nodes(&mut self, registry: &TaskRegistry, current_world: &WorldState) {
        if !self.active_nodes.is_empty() {
            return;
        }
        let seeds = self.possible_tasks(current_world, registry);
        for s in seeds {
            let Some(data) = registry.get_task(&s) else {
                continue;
            };
            self.active_nodes.push_back(Arc::new(Node {
                value: PlanNode {
                    task: Some(s),
                    world: current_world.clone().concat(data.postconditions()),
                    cost: data.cost(&current_world),
                    depth: 0,
                },
                parent: None,
            }));
        }
    }

    pub fn try_emit_single(&mut self, goal: &Goal) {
        let Some(valid) = self.valid_nodes.pop() else {
            return;
        };
        let plan = Self::unravel_plan(&valid);

        if let Some(prev_plan) = self.plans.get(&goal.name) {
            // ensure the plan we made is actually better than what was available
            if plan.cost > prev_plan.cost {
                return;
            }
        }

        self.plans.insert(goal.name.clone(), plan);
    }

    pub fn generate_single(
        &mut self,
        goal: &Goal,
        task_registry: &TaskRegistry,
        max_node_depth: Option<u32>,
    ) {
        // process a single node (so we can modify the dequeue without extra vecs to track)
        let Some(node) = self.active_nodes.pop_front() else {
            return;
        };
        if goal.requires.validate(&node.value.world) {
            // found a leaf! stop processing it
            eprintln!("Found Leaf Node: {:#?}", node.value);
            self.valid_nodes.push(node);
            return;
        }
        if node.value.depth >= max_node_depth.unwrap_or(u32::MAX) || self.has_recursion(&node) {
            return;
        }
        let tasks = self.possible_tasks(&node.value.world, task_registry);
        for t in tasks {
            if let Some(new_node) = Self::make_node(node.clone(), &t, task_registry) {
                self.active_nodes.push_front(Arc::new(new_node));
            }
        }
    }

    fn unravel_plan(leaf: &Arc<Node<PlanNode>>) -> Plan {
        let mut curr = leaf.clone();
        let mut sequence = Vec::<Task>::new();
        loop {
            let val = curr.value.clone();
            let Some(task) = val.task else {
                error!("Found a None task while unravelling task graph");
                continue;
            };
            sequence.push(task);
            let Some(next_curr) = curr.parent.clone() else {
                // effectively a do-while parent.is_some
                break;
            };
            curr = next_curr;
        }
        Plan {
            tasks: sequence.into(),
            cost: leaf.value.cost,
        }
    }

    // this is a total band-aid solution. Probably need a better way to coerce the plan to avoid repetitive tasks?
    fn has_recursion(&self, node: &Arc<Node<PlanNode>>) -> bool {
        let Some(ref parent) = node.parent else {
            return false;
        };
        let Some(ref parent2) = parent.parent else {
            return false;
        };
        let Some(ref parent3) = parent2.parent else {
            return false;
        };
        let t0 = node
            .value
            .task
            .as_ref()
            .and_then(|task| Some(task.name()))
            .unwrap_or("0".into());
        let t1 = parent
            .value
            .task
            .as_ref()
            .and_then(|task| Some(task.name()))
            .unwrap_or("0".into());
        let t2 = parent2
            .value
            .task
            .as_ref()
            .and_then(|task| Some(task.name()))
            .unwrap_or("0".into());
        let t4 = parent3
            .value
            .task
            .as_ref()
            .and_then(|task| Some(task.name()))
            .unwrap_or("0".into());

        // this only catches A-B-A-B patterns, not A-B-C-A-B-C patterns
        // goddamn I need a better solution
        t0 == t2 && t1 == t4
    }

    fn possible_tasks(&self, world: &WorldState, task_registry: &TaskRegistry) -> Vec<Task> {
        // self.available_tasks
        //     .clone()
        //     .into_iter()
        //     .filter(|p| task_registry.precon(p).unwrap_or_default().validate(world))
        //     .collect()
        let mut n_vec = Vec::new();
        for task in self.available_tasks.iter() {
            let Some(precon) = task_registry.precon(task) else {
                continue;
            };
            if precon.validate(world) {
                n_vec.push(task.clone());
            }
        }
        n_vec
    }
    fn make_node(
        parent: Arc<Node<PlanNode>>,
        task: &Task,
        registry: &TaskRegistry,
    ) -> Option<Node<PlanNode>> {
        let Some(data) = registry.get_task(task) else {
            return None;
        };
        let virtual_world = parent.value.world.concat(data.postconditions());
        Some(Node::<PlanNode> {
            value: PlanNode {
                task: Some(task.clone()),
                cost: parent.value.cost + data.cost(&virtual_world),
                world: virtual_world,
                depth: parent.value.depth + 1,
            },
            parent: Some(parent),
        })
    }
}

pub fn system_update_time_sliced_tree_gen(
    mut query: Query<(&mut TimeSlicedTreeGen, Option<&HtnAgentWorld>), With<HtnAgent>>,
    settings: Res<HtnSettings>,
    registry: Res<TaskRegistry>,
    world: Res<WorldState>,
) {
    let timer = Instant::now();
    for (mut sliced, agent_world) in query.iter_mut() {
        let active_world = match agent_world {
            Some(c) => world.concat(&c.0),
            None => world.to_owned(),
        };
        sliced.generate_for_duration(
            &registry,
            &active_world,
            settings.frame_processing_limit,
            settings.node_branch_limit,
        );

        if let Some(duration_limit) = settings.frame_processing_limit {
            if timer.elapsed() > duration_limit {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use bevy::prelude::Component;
    use goals::Goal;
    use plan_data::TimeSlicedTreeGen;

    use crate::prelude::*;

    #[derive(Component, Default)]
    struct TaskStub;

    #[test]
    fn single_task_planning() {
        let mut registry = TaskRegistry::new();
        registry.task::<TaskStub, _>(
            "test",
            Requirements::new().req_equals("hungry", true).build(),
            WorldState::new().add("hungry", false).build(),
            1.,
        );

        let goal = Goal::new(
            "Be Not Hungry",
            Requirements::new().req_equals("hungry", false).build(),
            1.0,
        );
        let mut gen =
            TimeSlicedTreeGen::new_initialized(vec![Task::primitive("test")], vec![goal.clone()]);
        // here the two limits are mainly to avoid execessive generation times
        gen.generate_to_completion(
            &registry,
            &WorldState::new().add("hungry", true).build(),
            Some(8),
        );

        let result = gen.plans.get(&goal.name);

        assert!(result.is_some());
        let plan = result.unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.cost, 1.0);
    }

    #[test]
    fn multi_task_planning() {
        let mut registry = TaskRegistry::new();
        registry.task::<TaskStub, _>(
            "goto_door",
            Requirements::new()
                .req_equals("room", "A")
                .req_equals("near_door", false)
                .build(),
            WorldState::new().add("near_door", true).build(),
            1.,
        );
        registry.task::<TaskStub, _>(
            "open_door",
            Requirements::new()
                .req_equals("near_door", true)
                .req_equals("door_open", false)
                .build(),
            // WorldState::new()
            //     .add("near_door", true)
            //     .add("door_open", false)
            //     .build(),
            WorldState::new().add("door_open", true).build(),
            1.,
        );
        registry.task::<TaskStub, _>(
            "walk_thru_door",
            Requirements::new()
                .req_equals("room", "A")
                .req_equals("door_open", true)
                .req_equals("near_door", true)
                .build(),
            WorldState::new().add("room", "B").build(),
            1.,
        );
        let goal = Goal::new(
            "Be in room B",
            Requirements::new().req_equals("room", "B").build(),
            1.0,
        );
        let mut gen = TimeSlicedTreeGen::new_initialized(
            vec![
                Task::primitive("goto_door"),
                Task::primitive("open_door"),
                Task::primitive("walk_thru_door"),
            ],
            vec![goal.clone()],
        );
        let initial_world = WorldState::new()
            .add("room", "A")
            .add("near_door", false)
            .add("door_open", false)
            .build();
        // here the two limits are mainly to avoid execessive generation times
        gen.generate_to_completion(&registry, &initial_world, Some(8));

        let result = gen.plans.get(&goal.name);

        assert!(result.is_some());
        let plan = result.unwrap();
        assert_eq!(plan.tasks.len(), 3);
        assert_eq!(plan.cost, 3.0);
    }
}
