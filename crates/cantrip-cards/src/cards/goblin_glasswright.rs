use cantrip_api::*;
use cantrip_mechanics::{prepare, tokens};
use cantrip_oracle as oracle;

/// "Create a Treasure token."
fn craft_with_pride(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)
}

/// Goblin Glasswright's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static CRAFT_WITH_PRIDE: CardDef = CardDef {
    name: "Craft with Pride",
    cost: mana("{R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], craft_with_pride),
    text: "Create a Treasure token. (It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&CRAFT_WITH_PRIDE),
    ..oracle::GOBLIN_GLASSWRIGHT
};

pub static EXTRA: &[&CardDef] = &[&CRAFT_WITH_PRIDE, &tokens::TREASURE];
