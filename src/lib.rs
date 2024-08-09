#![feature(trivial_bounds)]

pub mod data;
pub mod events;
pub mod execution;
pub mod orchestration;
pub mod planning;
pub mod tasks;

pub mod prelude {
    use bevy::app::App;
    use bevy::app::Plugin;

    pub use crate::data::*;
    pub use crate::execution::*;
    pub use crate::orchestration::*;
    pub use crate::planning::*;
    pub use crate::tasks::*;

    pub struct HtnPlanningPlugin {
        initial_world: Option<WorldState>,
        orchestrate: OrchestrateFor,
    }

    impl Plugin for HtnPlanningPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(self.initial_world.as_ref().cloned().unwrap_or_default());
            crate::data::plugin(app);
            crate::tasks::plugin(app);
            crate::planning::plugin(app);
            crate::orchestration::orchestrate_systems(app, &self.orchestrate);
        }
    }
    impl Default for HtnPlanningPlugin {
        fn default() -> Self {
            Self::new()
        }
    }
    impl HtnPlanningPlugin {
        pub fn new() -> Self {
            Self {
                initial_world: None,
                orchestrate: Default::default(),
            }
        }

        pub fn world(self, world: impl Into<WorldState>) -> Self {
            Self {
                initial_world: Some(world.into()),
                orchestrate: self.orchestrate,
            }
        }

        pub fn orchestrate(self, orch: OrchestrateFor) -> Self {
            Self {
                orchestrate: orch,
                initial_world: self.initial_world,
            }
        }
    }
}

#[cfg(test)]
mod tests {}
