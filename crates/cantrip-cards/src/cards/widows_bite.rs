use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets, teamwork};
use cantrip_oracle as oracle;

/// "Target creature gains deathtouch until end of turn." (CR 611.2a): the target of its own mode (CR 700.2c); nothing if it is
/// no longer a legal target as the spell resolves, the other mode still doing its part (CR 608.2b).
fn deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Deathtouch);
    }
    Ok(())
}

/// "Target creature gets -2/-2 until end of turn." (CR 611.2a, 704.5f): see `deathtouch`.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        pump(g, item.object, creature, -2, -2);
    }
    Ok(())
}

/// "Choose one. If this spell was cast using teamwork, choose both instead." (CR 700.2, 702.194b): with teamwork both modes, each
/// with its own target (CR 700.2c), done in printed order (CR 700.2d), so a creature that gets deathtouch and -2/-2 in the same
/// spell gets both.
static MODAL: ModalDef = modes(&[
    mode(deathtouch, "Target creature gains deathtouch until end of turn.").targets(&[targets::CREATURE]),
    mode(minus_two, "Target creature gets -2/-2 until end of turn.").targets(&[targets::CREATURE]),
])
.when_paid(2, 2);

/// Teamwork 3 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 3 or
/// more): announced before the modes, since how many are chosen is defined by it (CR 601.2b).
pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    optional_cost: Some(teamwork::optional(3)),
    labels: &["Cast without teamwork (choose one)", "Teamwork 3: Tap any number of creatures you control with total power 3 or more (choose both)"],
    ..oracle::WIDOWS_BITE
};
