use crate::execution::*;
use crate::planning::plan_data::system_update_time_sliced_tree_gen;
use crate::planning::{
    system_collect_agent_goals_from_providers, system_collect_agent_tasks_from_providers,
};
use bevy::{
    app::{App, Update},
    prelude::IntoSystemConfigs,
};

#[derive(Default)]
pub enum OrchestrateFor {
    /// Systems are orchestrated to maximize parallel processing
    #[default]
    ParallelProcessing,
    // Systems are chained so they all execute on maximal agents across a single frame
    FasterResponse,
    // No built-in orchestration, set it up yourself and even inject your own custom systems if you so choose!
    Custom,
}

pub(crate) fn orchestrate_systems(app: &mut App, style: &OrchestrateFor) {
    match style {
        OrchestrateFor::ParallelProcessing => {
            app.add_systems(
                Update,
                (
                    system_collect_agent_tasks_from_providers,
                    system_collect_agent_goals_from_providers,
                    system_extract_plans_for_unplanned_agents,
                    system_handle_agent_state_changes,
                    system_update_time_sliced_tree_gen,
                ), // no chaining means all systems run independently.
                   // This means some agents might not get a full processing sequence until a few frames later. Though it does allow beter multiprocessing
            );
        }
        OrchestrateFor::FasterResponse => {
            app.add_systems(
                Update,
                (
                    system_collect_agent_tasks_from_providers,
                    system_collect_agent_goals_from_providers,
                    system_extract_plans_for_unplanned_agents,
                    system_handle_agent_state_changes,
                    system_update_time_sliced_tree_gen,
                )
                    .chain(), // chaining ensures each system provides the requirements for the next for better response across frames
            );
        }
        OrchestrateFor::Custom => (),
    };
}
