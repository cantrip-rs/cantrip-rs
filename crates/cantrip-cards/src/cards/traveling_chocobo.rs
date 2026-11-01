use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{entering, subtype_spell};
use cantrip_oracle as oracle;

/// "If a land or Bird you control entering the battlefield causes a triggered ability of a permanent you control to
/// trigger, that ability triggers an additional time" (CR 603.2d, 603.6a): the entering permanent judged as it is on the
/// battlefield, under this permanent's controller; the Chocobo's own entering counts (the rulings).
fn land_or_bird_you_control_entering(q: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller)
        && entering(event).is_some_and(|permanent| {
            let c = q.chars(permanent);
            q.controller(permanent) == asker.controller
                && (c.is(CardType::Land) || c.has_subtype(q.words(asker.source).subtype(Subtype::Bird)))
        })
}

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and "you may play lands and cast Bird spells from
    // the top of your library" (CR 305.1, 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands, PlayRule::LibraryTopSpells(subtype_spell::<{ lands::sub(Subtype::Bird) }>, None)],
    additional_triggers: &[land_or_bird_you_control_entering],
    ..oracle::TRAVELING_CHOCOBO
};
