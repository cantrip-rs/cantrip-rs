use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::C).only_for(ManaRestriction::NOT_SPELLS_FROM_HAND),
        "{T}: Add {C}. This mana can't be spent to cast spells from your hand.",
    )],
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&tokens::SOUL_TETHER),
    ..oracle::HEARTWOOD_CRAFTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOUL_TETHER, &tokens::HEARTWOOD];
