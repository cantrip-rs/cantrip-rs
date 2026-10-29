use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// A Food artifact (CR 111.10b names the Food ability its tokens have; this card prints its own two): "{1}, {T}, Sacrifice this
/// artifact: Add one mana of any color." is a mana ability (CR 605.1a), its {1} and the sacrifice costs of the ability.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this artifact enters, draw a card."),
        mana_ability(cost("{1}, {T}").and(SACRIFICE_THIS), add_any_color(1), "{1}, {T}, Sacrifice this artifact: Add one mana of any color."),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), |g, item| effects::gain_life(g, item, 3), "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
    ],
    ..oracle::OMNI_CHEESE_PIZZA
};
