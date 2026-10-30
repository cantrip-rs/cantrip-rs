use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "create a tapped Mutavault token." A token created by name has the characteristics of the Oracle card of that name (CR 111.11,
/// and the card's ruling): Mutavault's own definition and both its abilities. The reminder text is no rules text (CR 207.2a),
/// so a text change of this card's changes nothing of it.
fn create_mutavault(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &super::mutavault::CARD, 1, true)?;
    Ok(())
}

/// Changeling is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        create_mutavault,
        "When this creature enters, create a tapped Mutavault token. (It's a land with \"{T}: Add {C}\" and \"{1}: This token becomes a 2/2 creature with all creature types until end of turn. It's still a land.\")",
    )],
    ..oracle::MUTABLE_EXPLORER
};
