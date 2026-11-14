use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, mana};
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

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{2}")),
            "Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    replacements: &[
        replacement(
            ReplacementKind::ManaProduction,
            if_land_tapped_mana_produces_applies,
            mana::produce_colorless_instead,
            "If a land is tapped for mana, it produces colorless mana instead of any other type.",
        ),
    ],
    ..oracle::RITUAL_OF_SUBDUAL
};
