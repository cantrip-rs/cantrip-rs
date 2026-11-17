use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "You may pay {W}{U}{B}{R}{G} rather than pay the mana cost for spells you cast" (CR 118.9, 601.2b): an alternative cost a static
/// ability gives to every spell its controller casts, from whatever zone and of whatever type, a card with no mana cost included
/// (CR 118.6a). It is the one alternative cost of that cast (CR 118.9a); additional costs and what raises or lowers the total
/// still apply (CR 118.9d, 601.2f); X is 0, as the cost paid is not the mana cost (CR 107.3b). An opponent's spells are not "spells
/// you cast" (`statics::spells_you_cast`).
pub static CARD: CardDef = CardDef {
    spell_alternatives: &[SpellAlternative {
        text: "Pay {W}{U}{B}{R}{G}",
        applies: statics::spells_you_cast,
        cost: InsteadOfManaCost::Mana(mana("{W}{U}{B}{R}{G}")),
        flash: false,
        kind: WayKind::Other,
    }],
    opening_hand: Some(statics::BEGIN_ON_THE_BATTLEFIELD),
    ..oracle::LEYLINE_OF_MUTATION
};
