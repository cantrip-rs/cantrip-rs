use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "If you gaining life causes a triggered ability of a permanent you control to trigger, that ability triggers an
/// additional time" (CR 603.2d). It counts each life-gain event, not each life point, and excludes delayed abilities.
fn gaining_life_triggers_again(_: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    matches!(event, Event::LifeChange { player, delta } if *player == asker.controller && *delta > 0)
        && triggering.of_a_permanent_controlled_by(asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_GAIN_LIFE, effects::plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on Dr. Crusher.")],
    additional_triggers: &[gaining_life_triggers_again],
    ..oracle::DR_BEVERLY_CRUSHER
};
