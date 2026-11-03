use cantrip_api::*;
use cantrip_mechanics::{poison, tokens};
use cantrip_oracle as oracle;

pub static SNAKE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[
        poison::on_damage_to_player::<1>(
            "Whenever this creature deals damage to a player, that player gets a poison counter.",
        ),
    ],
    text: "Whenever this creature deals damage to a player, that player gets a poison counter.",
    ..token!(Snake)
};

/// "{4}, {T}: Create a 1/1 colorless Snake artifact creature token. It has "Whenever this creature deals damage to a player, that player gets a poison counter."" (CR 608.2).
fn create_1_1_colorless_snake(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SNAKE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{4}").and(TAP),
            create_1_1_colorless_snake,
            "{4}, {T}: Create a 1/1 colorless Snake artifact creature token. It has \"Whenever this creature deals damage to a player, that player gets a poison counter.\"",
        ),
    ],
    ..oracle::SERPENT_GENERATOR
};

pub static EXTRA: &[&CardDef] = &[&SNAKE];
