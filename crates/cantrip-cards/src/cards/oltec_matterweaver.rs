use cantrip_api::*;
use cantrip_mechanics::{copy, targets, tokens, triggers::CREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

/// "Create a 1/1 colorless Gnome artifact creature token." (CR 111.1)
fn create_a_gnome(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GNOME, 1)?;
    Ok(())
}

/// "Whenever you cast a creature spell, choose one —" (CR 603.2, 700.2). The copy mode targets an artifact token you control,
/// so it can be chosen only when you control one (CR 700.2a).
static MODAL: ModalDef = modes(&[
    mode(create_a_gnome, "Create a 1/1 colorless Gnome artifact creature token."),
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target artifact token you control.")
        .targets(&[targets::ARTIFACT_TOKEN_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        CREATURE_SPELL_YOU_CAST,
        &MODAL,
        "Whenever you cast a creature spell, choose one — create a 1/1 colorless Gnome artifact creature token; or create a token that's a copy of target artifact token you control.",
    )],
    ..oracle::OLTEC_MATTERWEAVER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GNOME];
