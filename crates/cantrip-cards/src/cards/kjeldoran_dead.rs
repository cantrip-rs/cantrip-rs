use cantrip_api::*;
use cantrip_mechanics::{regenerate, sacrifice_chosen, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, sacrifice a creature." (CR 608.2, 701.21a).
fn sacrifice_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Creature)).map(drop)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_creature,
            "When this creature enters, sacrifice a creature.",
        ),
        ability(cost("{B}"), regenerate::this_effect, "{B}: Regenerate this creature."),
    ],
    ..oracle::KJELDORAN_DEAD
};
