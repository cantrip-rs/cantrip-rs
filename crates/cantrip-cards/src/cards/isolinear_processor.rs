use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add(Mana::C).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES),
            "{T}: Add {C}. Spend this mana only to cast an artifact spell or activate an ability of an artifact source.",
        ),
        ability(cost("{2}").and(TAP), cantrip_mechanics::effects::loot, "{2}, {T}: Draw a card, then discard a card."),
    ],
    ..oracle::ISOLINEAR_PROCESSOR
};
