use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Look at the top X cards of your library. Put one of them into your hand and the rest on the bottom of your library in a
/// random order." (CR 107.3, 401.4): X is the one chosen as the spell was cast, the trigger's `data[0]`. With X of 0 nothing is
/// looked at; a library with fewer cards shows what it has (CR 609.3).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, item.data[0] as usize, (1, 1), false, |_| true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_CAST_A_SPELL_WITH_X_IN_ITS_MANA_COST,
        look,
        "Whenever you cast a spell with {X} in its mana cost, look at the top X cards of your library. Put one of them into your hand and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::GEOMETERS_ARTHROPOD
};
