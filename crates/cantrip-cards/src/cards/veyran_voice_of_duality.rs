use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{MAGECRAFT, instant_or_sorcery_cast_or_copied};
use cantrip_oracle as oracle;

/// "If you casting or copying an instant or sorcery spell causes a triggered ability of a permanent you control to trigger,
/// that ability triggers an additional time." (CR 603.2d, 707.10): magecraft and any other ability that event triggers -
/// prowess for an instant, "whenever you cast a spell" - but not one an opponent's spell triggers.
fn you_casting_or_copying(q: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller)
        && instant_or_sorcery_cast_or_copied(q, event).is_some_and(|(_, player)| player == asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(MAGECRAFT, effects::this_gets::<1, 1>, "Magecraft \u{2014} Whenever you cast or copy an instant or sorcery spell, Veyran gets +1/+1 until end of turn.")],
    additional_triggers: &[you_casting_or_copying],
    ..oracle::VEYRAN_VOICE_OF_DUALITY
};
