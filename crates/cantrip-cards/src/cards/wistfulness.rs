use cantrip_api::*;
use cantrip_mechanics::{evoke, spent, targets, triggers::THIS_ENTERS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[evoke::way(Cost::mana(mana("{G/U}{G/U}")), "Evoke {G/U}{G/U}")],
    abilities: &[
        evoke::sacrifice_trigger("When Wistfulness enters, if its evoke cost was paid, its controller sacrifices it."),
        triggered(THIS_ENTERS, effects::exile_first_target, "When this creature enters, if {G}{G} was spent to cast it, exile target artifact or enchantment an opponent controls.")
            .targets(&[targets::ARTIFACT_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS])
            .condition(spent::two::<{ Mana::G as u8 }>),
        triggered(THIS_ENTERS, effects::draw_then_discard::<2, 1>, "When this creature enters, if {U}{U} was spent to cast it, draw two cards, then discard a card.")
            .condition(spent::two::<{ Mana::U as u8 }>),
    ],
    ..oracle::WISTFULNESS
};
