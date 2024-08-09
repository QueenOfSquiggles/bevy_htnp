use bevy::{app::App, prelude::Component};

use crate::tasks::Task;

use super::goals::Goal;

pub(crate) fn plugin(app: &mut App) {
    use bevy_trait_query::RegisterExt;
    app.register_component_as::<dyn TaskProvider, StaticTaskProvider>();
    app.register_component_as::<dyn GoalProvider, StaticGoalProvider>();
}

#[bevy_trait_query::queryable]
/// Implement this trait on a component to allow it to provide a set of tasks to an HTN agent
pub trait TaskProvider {
    fn tasks(&self) -> Vec<Task>;
}

#[bevy_trait_query::queryable]
/// Implement this trait on a component to allow it to provide a set of goals to an HTN agent
pub trait GoalProvider {
    fn goals(&self) -> Vec<Goal>;
}

#[derive(Component)]
pub struct StaticTaskProvider(Vec<Task>);

#[derive(Component)]
pub struct StaticGoalProvider(Vec<Goal>);

impl TaskProvider for StaticTaskProvider {
    fn tasks(&self) -> Vec<Task> {
        self.0.clone()
    }
}

impl GoalProvider for StaticGoalProvider {
    fn goals(&self) -> Vec<Goal> {
        self.0.clone()
    }
}

impl StaticTaskProvider {
    pub fn new(tasks: Vec<Task>) -> Self {
        Self(tasks)
    }
}

impl StaticGoalProvider {
    pub fn new(goals: Vec<Goal>) -> Self {
        Self(goals)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::prelude::*;

    use crate::planning::{
        system_collect_agent_goals_from_providers, system_collect_agent_tasks_from_providers,
        HtnAgent,
    };
    use crate::{data::Requirements, tasks::Task};

    #[test]
    fn test_provider_tasks() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        super::plugin(&mut app); // registration still needs to be done to make things work here!
        app.add_systems(Update, system_collect_agent_tasks_from_providers);
        let agent = app
            .world_mut()
            .spawn((
                HtnAgent::default(),
                StaticTaskProvider::new(vec![
                    Task::primitive("A"),
                    Task::primitive("B"),
                    Task::primitive("C"),
                ]),
            ))
            .id();
        app.update();
        let data = app
            .world()
            .get::<HtnAgent>(agent)
            .expect("Failed to find agent component!");

        assert_eq!(data.available_tasks.len(), 3);
        assert_eq!(data.available_tasks[0], Task::primitive("A"));
        assert_eq!(data.available_tasks[1], Task::primitive("B"));
        assert_eq!(data.available_tasks[2], Task::primitive("C"));
    }
    #[test]
    fn test_provider_goals() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        super::plugin(&mut app);
        app.add_systems(Update, system_collect_agent_goals_from_providers);
        let agent = app
            .world_mut()
            .spawn((
                HtnAgent::default(),
                StaticGoalProvider::new(vec![
                    Goal::new("A", Requirements::new(), 1.0),
                    Goal::new("B", Requirements::new(), 1.0),
                    Goal::new("C", Requirements::new(), 1.0),
                ]),
            ))
            .id();
        app.update();
        let data = app
            .world()
            .get::<HtnAgent>(agent)
            .expect("Failed to find agent component!");

        assert_eq!(data.goals.len(), 3);
        assert_eq!(data.goals[0].name, "A");
        assert_eq!(data.goals[1].name, "B");
        assert_eq!(data.goals[2].name, "C");
    }

    #[test]
    fn test_custom_provider_tasks() {
        use bevy_trait_query::RegisterExt;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, system_collect_agent_tasks_from_providers);
        app.register_component_as::<dyn TaskProvider, CustomTaskProvider>();
        let agent = app
            .world_mut()
            .spawn((HtnAgent::default(), CustomTaskProvider))
            .id();
        app.update();
        let data = app
            .world()
            .get::<HtnAgent>(agent)
            .expect("Failed to find agent component!");
        assert_eq!(data.available_tasks.len(), 1);
        assert_eq!(data.available_tasks[0], Task::primitive("something"))
    }
    #[test]
    fn test_custom_provider_goals() {
        use bevy_trait_query::RegisterExt;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, system_collect_agent_goals_from_providers);
        app.register_component_as::<dyn GoalProvider, CustomGoalProvider>();
        let agent = app
            .world_mut()
            .spawn((HtnAgent::default(), CustomGoalProvider))
            .id();
        app.update();
        let data = app
            .world()
            .get::<HtnAgent>(agent)
            .expect("Failed to find agent component!");
        assert_eq!(data.goals.len(), 1);
        assert_eq!(data.goals[0].name, "something");
    }

    #[derive(Component)]
    struct CustomTaskProvider;
    #[derive(Component)]
    struct CustomGoalProvider;

    impl TaskProvider for CustomTaskProvider {
        fn tasks(&self) -> Vec<Task> {
            vec![Task::primitive("something")]
        }
    }

    impl GoalProvider for CustomGoalProvider {
        fn goals(&self) -> Vec<Goal> {
            vec![Goal::new("something", Requirements::new(), 1.0)]
        }
    }
}
