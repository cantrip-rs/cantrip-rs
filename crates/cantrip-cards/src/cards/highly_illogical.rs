use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell with mana value 2 or less. If this spell was kicked, instead counter target spell."
/// Exactly one target is announced: kicker replaces the restricted slot (CR 702.33g, 601.2c, 701.6).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for spell in targets::legal_objects(g, item) {
        g.counter(spell);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{1}{U}")),
    spell: spell(
        &[
            target(targets::spell_with_mana_value_at_most::<2>).if_kicked(0, 0),
            targets::SPELL.between(0, 0).if_kicked(1, 1),
        ],
        counter,
    ),
    ..oracle::HIGHLY_ILLOGICAL
};
