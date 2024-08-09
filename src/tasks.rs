use crate::data::{Requirements, WorldState};
use bevy::{ecs::system::EntityCommands, prelude::*, utils::HashMap};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

pub(crate) fn plugin(app: &mut App) {
    app.insert_resource(TaskRegistry::default());
}

pub trait TaskData: Sync + Send {
    fn preconditions(&self) -> &Requirements;
    fn postconditions(&self) -> &WorldState;
    fn add(&self, entity: &mut EntityCommands);
    fn remove(&self, entity: &mut EntityCommands);
    fn cost(&self, world: &WorldState) -> f32;
}

/// We store tasks in an atomic ref-counted box. This means they are thread-safe dynamic allocations that are explicitly read-only.
pub type TaskStorage = Arc<Box<dyn TaskData>>;

#[derive(Resource, Default)]
pub struct TaskRegistry(pub HashMap<String, TaskStorage>);

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_task(&self, task: &Task) -> Option<&TaskStorage> {
        let Task::Primitive(name) = task else {
            return None;
        };
        if let Some(task) = self.0.get(name) {
            return Some(task);
        }
        None
    }
    pub fn get_named(&self, task: &String) -> Option<&TaskStorage> {
        self.0.get(task)
    }

    pub fn task<C, S>(&mut self, name: S, precon: Requirements, postcon: WorldState, cost: f32)
    where
        S: Into<String>,
        C: Component + Default,
    {
        let comp = SimpleTaskData::<C>::new(precon, postcon, cost);
        self.0.insert(name.into(), Arc::new(Box::new(comp)));
    }

    /// utility to more easily get both pre and post conditions for situations where both are needed
    pub fn pre_and_postcon(&self, task: &Task) -> Option<(Requirements, WorldState)> {
        let pre = self.precon(task);
        let post = self.postcon(task);
        if pre.is_none() || post.is_none() {
            return None;
        }
        Some((pre.unwrap(), post.unwrap()))
    }

    pub fn precon(&self, task: &Task) -> Option<Requirements> {
        match task {
            Task::Primitive(name) => {
                if let Some(data) = self.get_named(name) {
                    return Some(data.preconditions().clone());
                }
                None
            }
            Task::Macro(tasks, _) => {
                let mut req = Requirements::new();
                for t in tasks
                    .iter()
                    .map(|t| t.decompose())
                    .rev()
                    .reduce(|agg, item| {
                        agg.into_iter()
                            .chain(item.into_iter())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default()
                {
                    let Some(data) = self.get_named(&t) else {
                        return None;
                    };
                    req = req.unmet_requirements(data.postconditions());
                    req.append(data.preconditions());
                }
                Some(req)
            }
        }
    }

    pub fn postcon(&self, task: &Task) -> Option<WorldState> {
        match task {
            Task::Primitive(name) => {
                if let Some(data) = self.get_named(name) {
                    return Some(data.postconditions().clone());
                }
                None
            }
            Task::Macro(tasks, _) => {
                let mut context = WorldState::new();
                for t in tasks
                    .iter()
                    .map(|t| t.decompose())
                    .rev()
                    .reduce(|agg, item| {
                        agg.into_iter()
                            .chain(item.into_iter())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default()
                {
                    let Some(data) = self.get_named(&t) else {
                        return None;
                    };
                    context = data.preconditions().consume(&context);
                    context.append(data.postconditions());
                }
                Some(context)
            }
        }
    }

    pub fn custom_task<S>(&mut self, name: S, data: Box<dyn TaskData>)
    where
        S: Into<String>,
    {
        self.0.insert(name.into(), Arc::new(data));
    }
}

/// For instances where pre and post conditions are static and the task is accomplished through a default instance of a component, this can be used to make creation of new tasks much easier.
struct SimpleTaskData<C>
where
    C: Component,
{
    precon: Requirements,
    postcon: WorldState,
    cost: f32,
    phantom: PhantomData<C>,
}

impl<C> SimpleTaskData<C>
where
    C: Component + Default,
{
    fn new(precon: Requirements, postcon: WorldState, cost: f32) -> Self {
        Self {
            precon,
            postcon,
            phantom: PhantomData,
            cost,
        }
    }
}

impl<C> TaskData for SimpleTaskData<C>
where
    C: Component + Default,
{
    fn preconditions(&self) -> &Requirements {
        &self.precon
    }

    fn postconditions(&self) -> &WorldState {
        &self.postcon
    }

    fn add(&self, entity: &mut EntityCommands) {
        entity.insert(C::default());
    }

    fn remove(&self, entity: &mut EntityCommands) {
        entity.remove::<C>();
    }

    fn cost(&self, _: &WorldState) -> f32 {
        self.cost
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    Primitive(String),
    Macro(Vec<Task>, String),
}

impl Task {
    pub fn decompose_iter(iter: impl Iterator<Item = Task>) -> Vec<String> {
        iter.map(|t| t.decompose())
            .reduce(|agg, item| agg.into_iter().chain(item.into_iter()).collect())
            .unwrap_or_default()
    }

    pub fn name(&self) -> String {
        match self {
            Task::Primitive(name) => name,
            Task::Macro(_, name) => name,
        }
        .clone()
    }
    pub fn primitive(name: impl Into<String>) -> Self {
        Self::Primitive(name.into())
    }

    pub fn macro_(set: impl Iterator<Item = Task>, name: String) -> Self {
        Task::Macro(set.collect(), name)
    }
    pub fn decompose(&self) -> Vec<String> {
        match self {
            Task::Primitive(name) => {
                vec![name.clone()]
            }
            Task::Macro(m, _) => m
                .iter()
                .map(|p| p.decompose())
                .reduce(|agg, item| {
                    let mut n_agg = agg.clone();
                    for i in item {
                        n_agg.push(i);
                    }
                    n_agg
                })
                .unwrap_or_default(),
        }
    }
}
