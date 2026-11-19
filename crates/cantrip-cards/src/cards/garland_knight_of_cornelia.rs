use cantrip_api::*;
use cantrip_mechanics::triggers::{this_dies_about_the_card, NONCREATURE_SPELL_YOU_CAST};
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

/// "Return this card from your graveyard to the battlefield transformed" (CR 712.14a, 400.7): the ability's own source, put
/// onto the battlefield under its controller with its back face up; nothing if it is no longer the object the ability came from
/// (the core skips a card that changed zones).
fn return_this_card_transformed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_onto_battlefield_transformed(item.object, item.controller, false)?;
    Ok(())
}


/// Flying is a core keyword read from its characteristics.
pub static BACK: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, this_dies_about_the_card), library::put_subject_on_the_bottom, "When Chaos dies, put it on the bottom of its owner's library.")],
    ..oracle::CHAOS_THE_ENDLESS
};

/// The return works only from the graveyard (CR 113.6m), where the card is not a creature.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(NONCREATURE_SPELL_YOU_CAST, |g, item| effects::surveil(g, item, 1), "Whenever you cast a noncreature spell, surveil 1."),
        ability(
            cost("{3}{B}{B}{R}{R}"),
            return_this_card_transformed,
            "{3}{B}{B}{R}{R}: Return this card from your graveyard to the battlefield transformed. Activate only as a sorcery.",
        )
        .from(ZoneKind::Graveyard)
        .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::GARLAND_KNIGHT_OF_CORNELIA
};
