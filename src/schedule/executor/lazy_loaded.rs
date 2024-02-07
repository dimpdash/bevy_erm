#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use std::panic::AssertUnwindSafe;

use crate::{
    schedule::{BoxedCondition, ExecutorKind, SystemExecutor, SystemSchedule},
    world::World,
};

/// A variant of [`SingleThreadedExecutor`](crate::schedule::SingleThreadedExecutor) that calls
/// [`apply_deferred`](crate::system::System::apply_deferred) immediately after running each system.
#[derive(Default)]
pub struct LazyLoadedExecutor {
}

impl SystemExecutor for LazyLoadedExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Simple
    }

    fn set_apply_final_deferred(&mut self, _: bool) {
        // do nothing. simple executor does not do a final sync
    }

    fn init(&mut self, _schedule: &SystemSchedule) {
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        for system_index in 0..schedule.systems.len() {
            let system = &mut schedule.systems[system_index];
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                system.run((), world);
            }));
            if let Err(payload) = res {
                eprintln!("Encountered a panic in system `{}`!", &*system.name());
                std::panic::resume_unwind(payload);
            }

            system.apply_deferred(world);
        }

    }
}

impl LazyLoadedExecutor {
    /// Creates a new simple executor for use in a [`Schedule`](crate::schedule::Schedule).
    /// This calls each system in order and immediately calls [`System::apply_deferred`](crate::system::System::apply_deferred).
    pub const fn new() -> Self {
        Self {
        }
    }
}

fn evaluate_and_fold_conditions(conditions: &mut [BoxedCondition], world: &mut World) -> bool {
    // not short-circuiting is intentional
    #[allow(clippy::unnecessary_fold)]
    conditions
        .iter_mut()
        .map(|condition| condition.run((), world))
        .fold(true, |acc, res| acc && res)
}
