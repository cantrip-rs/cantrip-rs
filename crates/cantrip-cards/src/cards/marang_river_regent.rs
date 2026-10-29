use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The Omen (CR 720): cast from the hand as an Omen instant for {3}{U}; as it resolves it is shuffled into its owner's
/// library (CR 720.3d, the core's).
pub static COIL_AND_CATCH: CardDef = CardDef {
    spell: spell(&[], effects::draw_three_then_discard),
    ..oracle::COIL_AND_CATCH
};

/// "Return up to two other target nonland permanents to their owners' hands." Those still legal go together (CR 608.2b,
/// 400.7); none chosen, nothing happens.
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned = targets::legal_objects(g, item);
    g.return_to_hand(&returned);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_them,
        "When this creature enters, return up to two other target nonland permanents to their owners' hands.",
    )
    .targets(&[up_to(2, targets::other_nonland_permanent_filter)])],
    omen: Some(&COIL_AND_CATCH),
    ..oracle::MARANG_RIVER_REGENT
};

pub static EXTRA: &[&CardDef] = &[&COIL_AND_CATCH];
