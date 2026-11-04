use cantrip_api::*;
use cantrip_mechanics::{effects, select, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you lose 1 life and create a Treasure token."
fn lose_life_and_create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::lose_life_and_create_token(g, item, 1, &tokens::TREASURE)
}

/// "Whenever this creature attacks, you may sacrifice another creature or artifact. If you do, surveil 2." The sacrifice is chosen
/// as the ability resolves and may be declined (CR 608.2d, 701.21a); the surveil happens only if a permanent was actually
/// sacrificed (CR 608.2c), such as the Treasure it made.
fn sacrifice_to_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::another_creature_or_artifact_permanents(select::up_to(1));
    if !effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        effects::surveil(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, lose_life_and_create_a_treasure, "When this creature enters, you lose 1 life and create a Treasure token."),
        triggered(
            triggers::THIS_ATTACKS,
            sacrifice_to_surveil,
            "Whenever this creature attacks, you may sacrifice another creature or artifact. If you do, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
        ),
    ],
    ..oracle::NAMAZU_TRADER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
