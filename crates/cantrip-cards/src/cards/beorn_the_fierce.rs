use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{lands, targets};
use cantrip_oracle as oracle;

/// "Put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if
/// you control three or more Bears, draw two cards." (CR 122.1b, 205.1b, 608.2c): the new type has no duration (the ruling), so
/// it outlasts Beorn, and the Bears counted are those the layers give now, the creature just made one included, the type read
/// through the words of this card's text (CR 612). With no target chosen the rest still happens; with the one target gone
/// the ability is not resolved at all (CR 608.2b).
fn combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let bear = g.words(item.object).subtype(Subtype::Bear);
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::Trample, 1);
        g.end_sentence();
        g.add_effect(item.object, creature, EffectKind::AddSubtype(bear), Duration::Indefinite);
        g.end_sentence();
    }
    let bears = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(bear)).count();
    if bears >= 3 {
        g.draw(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Bear) }>,
        plus_pt::<2, 2>,
        "Other Bears you control get +2/+2.",
    )],
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        combat,
        "At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL.optional()])],
    ..oracle::BEORN_THE_FIERCE
};
