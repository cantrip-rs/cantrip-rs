use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{have_haste, itself_while_you_control_another};
use cantrip_oracle as oracle;

/// "Another Goblin" for the sacrifice (CR 701.21a, 109.1): a permanent other than this creature with the creature type Goblin, read
/// through the words of this card's text (CR 612). The permanents a cost can sacrifice are the paying player's own.
fn another_goblin(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // "{T}, Sacrifice another Goblin: Add {B}{R}." A mana ability (CR 605.1a): no target, it adds mana, and it is no loyalty ability.
        mana_ability(TAP.and(sacrifice(1, another_goblin)), add_each(&[Mana::B, Mana::R]), "{T}, Sacrifice another Goblin: Add {B}{R}."),
    ],
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_control_another::<{ lands::sub(Subtype::Goblin) }>,
        have_haste,
        "This creature has haste as long as you control another Goblin.",
    )],
    ..oracle::BOLGS_COMPANY
};
