use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "A creature that dealt damage this turn" (CR 120.2, 118.8): a creature the caster controls that has dealt damage to anything
/// this turn, combat or not, whatever has become of what it hit. The object is the one that dealt it (CR 400.7).
fn creature_that_dealt_damage_this_turn(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && history::dealt_damage_this_turn(q.history(), id)
}

/// "Draw three cards. Each opponent loses 3 life and you gain 3 life."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    effects::each_opponent_loses_life_you_gain_life::<3>(g, item)
}

pub static CARD: CardDef = CardDef {
    additional_cost: Some(sacrifice(1, creature_that_dealt_damage_this_turn)),
    spell: spell(&[], effect),
    ..oracle::TREACHEROUS_GREED
};
