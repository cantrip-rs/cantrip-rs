use cantrip_api::*;
use cantrip_mechanics::{attach, statics};
use cantrip_oracle as oracle;

/// "As long as equipped creature's toughness is greater than its power, it assigns combat damage equal to its toughness
/// rather than its power" (CR 510.1a): the creature this Equipment is attached to, read with its toughness after the +0/+1.
fn equipped_creature_with_toughness_over_power(q: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    let chars = q.chars(creature);
    q.state().obj(asker.source).attached_to == Some(creature) && chars.toughness > chars.power
}

pub static CARD: CardDef = CardDef {
    abilities: &[attach::equip(mana("{1}"), "Equip {1}")],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<0, 1>, "Equipped creature gets +0/+1.")],
    damage_by_toughness: &[equipped_creature_with_toughness_over_power],
    ..oracle::BARK_OF_DORAN
};
