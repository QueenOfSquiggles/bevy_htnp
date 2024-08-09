use bevy::prelude::{Commands, Event, Trigger};

use crate::prelude::{HtnAgentCurrentTask, HtnAgentPlan, HtnAgentState};

#[derive(Event)]
pub struct HtnPlanInvalidated;

pub fn observer_handle_invalidated_plan(
    trigger: Trigger<HtnPlanInvalidated>,
    mut commands: Commands,
) {
    commands
        .entity(trigger.entity())
        .remove::<(HtnAgentCurrentTask, HtnAgentState, HtnAgentPlan)>();
}
