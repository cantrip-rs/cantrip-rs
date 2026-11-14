use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {U} or {C}{U}. Spend this mana only to pay cumulative upkeep costs." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add_one_bundle_of(
                    &[ManaPool([0, 1, 0, 0, 0, 0]), ManaPool([0, 1, 0, 0, 0, 1])],
                    0,
                )
                .only_for(ManaRestriction::CumulativeUpkeep),
            "{T}: Add {U} or {C}{U}. Spend this mana only to pay cumulative upkeep costs.",
        ),
    ],
    labels: &["Add {U}", "Add {C}{U}"],
    ..oracle::ADARKAR_UNICORN
};
