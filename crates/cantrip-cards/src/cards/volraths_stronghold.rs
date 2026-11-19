use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature card from your graveyard" (CR 115.1).
fn creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{1}{B}, {T}"), effects::put_first_target_on_top_of_library, "{1}{B}, {T}: Put target creature card from your graveyard on top of your library.")
            .targets(&[target(creature_card_in_your_graveyard)]),
    ],
    ..oracle::VOLRATHS_STRONGHOLD
};
