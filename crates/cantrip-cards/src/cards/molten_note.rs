use cantrip_api::*;
use cantrip_mechanics::{damage_target, flashback, targets};
use cantrip_oracle as oracle;

/// "Molten Note deals damage to target creature equal to the amount of mana spent to cast this spell. Untap all creatures you
/// control": the mana spent (CR 601.2h) is everything paid for the spell, X, colored and any flashback cost alike, read from the
/// spell's own object. With the one target illegal the spell is not resolved at all (CR 608.2b), so nothing untaps.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spent = g.obj(item.object).spent.total();
    damage_target(g, item, 0, spent as u16);
    for creature in g.permanents_of(item.controller) {
        if g.chars(creature).is(CardType::Creature) {
            g.untap(creature);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ways: &[flashback::way(cost("{6}{R}{W}"), "Flashback {6}{R}{W}")],
    ..oracle::MOLTEN_NOTE
};
