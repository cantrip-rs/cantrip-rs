use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "A Room" (CR 205.3h), read through the words of the card whose text says it (CR 612): a permanent with the subtype.
fn room(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Room))
}

/// "This creature deals 2 damage to each opponent. Draw a card." (CR 120.3, 121.1).
fn deals_two_damage_to_each_opponent_and_draws(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 2);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(sacrifice(1, room)),
        deals_two_damage_to_each_opponent_and_draws,
        "{T}, Sacrifice a Room: This creature deals 2 damage to each opponent. Draw a card.",
    )],
    ..oracle::INTRUDING_SOULRAGER
};
