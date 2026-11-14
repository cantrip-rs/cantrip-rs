use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, text_change};
use cantrip_oracle as oracle;

/// "Choose a land type. Target creature you control gains snow landwalk of the chosen type until end of turn. Return that
/// creature to its owner's hand at the beginning of the next end step." The land type is chosen as the ability resolves
/// (CR 608.2d), by its controller, from every land type (CR 205.3i), not only the basic ones (the 2014-02-01 ruling: "you
/// choose a land subtype"). The creature gains the snow landwalk naming it (CR 702.14c: it can't be blocked as long as the
/// defending player controls a snow land of that type), a layer 6 effect for the rest of the turn (CR 611.2a, 613.1f).
/// Then a delayed triggered ability returns that object to its owner's hand at the beginning of the next end step (CR
/// 603.7a), and nothing if it has left the battlefield by then (CR 603.7c, 400.7).
fn guide(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let chosen = text_change::one_subtype(g, item.controller, item.object, SubtypeSet::LAND_TYPES.iter().collect(), WordUse::Chosen)?;
    // Every land type offered has its snow landwalk.
    let Some(walk) = Keyword::snow_landwalk_of(chosen) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, walk);
    delayed::at_next_end_step(g, item, 0, Target::Object(creature));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{R}, {T}"), guide,
        "{2}{R}, {T}: Choose a land type. Target creature you control gains snow landwalk of the chosen type until end of turn. Return that creature to its owner's hand at the beginning of the next end step.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    delayed: &[delayed_trigger(delayed::return_subject_to_hand, "Return that creature to its owner's hand.")],
    ..oracle::BARBARIAN_GUIDES
};
