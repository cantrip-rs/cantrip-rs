use cantrip_api::*;
use cantrip_mechanics::cast_from_your_graveyard;
use cantrip_mechanics::history::creatures_died_this_turn_that;
use cantrip_oracle as oracle;

/// "You may cast this card from your graveyard if a non-Zombie creature died this turn" (CR 601.2, 700.4): this card, by its
/// owner, once a creature that was not a Zombie as it died (CR 608.2h) has died since the turn began.
fn when_a_non_zombie_creature_died(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    let zombie = q.words(asker.source).subtype(Subtype::Zombie);
    cast_from_your_graveyard::this_card_by_its_owner(q, asker, card, player)
        && creatures_died_this_turn_that(q, None, |died| !died.subtypes.has(zombie)) > 0
}

/// "If you do, it enters with a +1/+1 counter on it" (CR 122.1, 614.1c, 601.3): only when it was cast by this card's own
/// permission, not by another that lets it be cast from the graveyard.
fn enters_with_plus_one_counter_if_cast_this_way(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    cast_from_your_graveyard::enters_with_counter_if_cast_by_it(g, entering, CounterKind::PlusOne)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_plus_one_counter_if_cast_this_way),
    exile_casts: &[ExileCast { allows: when_a_non_zombie_creature_died, ..cast_from_your_graveyard::PERMISSION }],
    ..oracle::UNDEAD_SPRINTER
};
