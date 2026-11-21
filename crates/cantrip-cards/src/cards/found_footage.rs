use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Face-down creatures your opponents control" (CR 708.5): asked of each face-down permanent as it is now, while Found
/// Footage is on the battlefield. Face-down spells stay hidden (the ruling): the rule is about permanents alone.
fn creature_an_opponent_controls(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.state().opponents(asker.controller).any(|p| p == q.state().obj(id).controller) && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtFaceDown(creature_an_opponent_controls)],
    abilities: &[ability(Cost::mana(mana("{2}")).and(SACRIFICE_THIS), effects::surveil_2_then_draw_a_card, "{2}, Sacrifice this artifact: Surveil 2, then draw a card.")],
    ..oracle::FOUND_FOOTAGE
};
