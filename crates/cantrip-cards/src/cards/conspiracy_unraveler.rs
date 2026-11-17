use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "You may collect evidence 10 rather than pay the mana cost for spells you cast" (CR 118.9, 601.2b, 701.59a): an alternative cost a
/// static ability gives to every spell its controller casts, from whatever zone and of whatever type, a card with no mana cost
/// included (CR 118.6a). It is the one alternative cost of that cast (CR 118.9a); additional costs and what raises or lowers the
/// total still apply (CR 118.9d, 601.2f); X is 0, as the cost paid is not the mana cost (CR 107.3b). Collecting evidence 10 exiles
/// cards with total mana value 10 or greater from the caster's graveyard (CR 701.59a) and cannot be paid with less. An opponent's
/// spells are not "spells you cast" (`statics::spells_you_cast`).
pub static CARD: CardDef = CardDef {
    spell_alternatives: &[SpellAlternative {
        text: "Collect evidence 10",
        applies: statics::spells_you_cast,
        cost: InsteadOfManaCost::Cost(Cost { collect_evidence: 10, ..Cost::NONE }),
        flash: false,
        kind: WayKind::Other,
    }],
    ..oracle::CONSPIRACY_UNRAVELER
};
