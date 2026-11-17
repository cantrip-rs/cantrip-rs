use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_DIES;
use cantrip_oracle as oracle;

/// "Players can't cast spells from graveyards" (CR 101.2, 601.3): anyone's, from any graveyard. Playing a land from one
/// is not casting (CR 305.1).
fn no_casting_from_graveyards(_: &dyn Query, _: &Asker, _: PlayerId, from: ZoneKind) -> bool {
    from != ZoneKind::Graveyard
}

/// "... or activate abilities of cards in graveyards" (CR 602.5): an activated ability of a card that is in a graveyard
/// (unearth, embalm, scavenge). Triggered abilities are not activated.
fn no_activating_in_graveyards(q: &dyn Query, _: &Asker, _: PlayerId, source: ObjectId, _: &ActivatedDef) -> bool {
    !q.obj(source).zone.is(ZoneKind::Graveyard)
}

/// "Whenever a creature dies, you gain 1 life." A leaves-the-battlefield ability, looked back on (CR 603.10a): a creature
/// dying as Ashes of the Abhorrent leaves with it still triggers it.
fn gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(CREATURE_DIES, gain_one, "Whenever a creature dies, you gain 1 life.")],
    cast_rules: &[no_casting_from_graveyards],
    activation_rules: &[no_activating_in_graveyards],
    ..oracle::ASHES_OF_THE_ABHORRENT
};
