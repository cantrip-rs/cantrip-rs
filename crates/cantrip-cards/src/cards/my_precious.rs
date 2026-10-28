use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip_cost, equipped_creature_cant_be_blocked};
use cantrip_mechanics::statics::have_hexproof;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "As an additional cost to cast this spell, sacrifice a creature." (CR 118.8) "Draw two cards."
pub static ALLURE_OF_POWER: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    additional_cost: Some(Cost::sacrifice(1, targets::filters::creature)),
    ..oracle::ALLURE_OF_POWER
};

/// "Equipped creature has hexproof and can't be blocked." (CR 702.11, 509.1b): "has" governs hexproof only, so "can't be blocked"
/// is a rule of the Equipment's, not an ability the creature gains (`attach::equipped_creature_cant_be_blocked`).
/// "Equip\u{2014}{2}, Pay 2 life." (CR 702.6a): an equip ability whose cost is mana and life.
pub static CARD: CardDef = CardDef {
    abilities: &[equip_cost(cost("{2}").and(life(2)), "Equip\u{2014}{2}, Pay 2 life.")],
    statics: &[static_ability(Layer::Ability, attached_object, have_hexproof, "Equipped creature has hexproof.")],
    block_rules: &[equipped_creature_cant_be_blocked],
    adventure: Some(&ALLURE_OF_POWER),
    ..oracle::MY_PRECIOUS
};

pub static EXTRA: &[&CardDef] = &[&ALLURE_OF_POWER];
