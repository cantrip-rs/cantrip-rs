use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_oracle as oracle;

/// "If a land is tapped for mana, it produces colorless mana instead of any other type." (CR 604.1).
fn if_land_tapped_mana_produces_applies(
    q: &dyn Query,
    _: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::ManaProduction(m) = proposal else { return false };
    let id = m.permanent;
    if q.chars(id).is(CardType::Land) {
        return true;
    }
    false
}

/// "If a land is tapped for mana, it produces colorless mana instead of any other type." (CR 604.1).
fn if_land_tapped_mana_produces_apply(
    _: &dyn Query,
    _: &Asker,
    proposal: &mut Proposal,
) {
    let seen = *proposal;
    let Proposal::ManaProduction(_) = &seen else { return };
    let Proposal::ManaProduction(m) = proposal else { return };
    let total = m.total();
    m.mana = ManaPool::default();
    m.mana.0[Mana::C as usize] = total;
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Mana(ManaCost { snow: 0, generic : 2, x
            : 0, symbols : [0, 0, 0, 0, 0, 0], hybrid : [None, None, None, None, None,
            None], twobrid : [0, 0, 0, 0, 0], phyrexian : [0, 0, 0, 0, 0] }),
            "Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    replacements: &[
        replacement(
            ReplacementKind::ManaProduction,
            if_land_tapped_mana_produces_applies,
            if_land_tapped_mana_produces_apply,
            "If a land is tapped for mana, it produces colorless mana instead of any other type.",
        ),
    ],
    ..oracle::RITUAL_OF_SUBDUAL
};
