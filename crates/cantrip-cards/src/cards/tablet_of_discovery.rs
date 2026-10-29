use cantrip_api::*;
use cantrip_mechanics::library::mill_a_card_and_play_it_this_turn;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, mill_a_card_and_play_it_this_turn, "When this artifact enters, mill a card. You may play that card this turn."),
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        mana_ability(TAP, add_n(Mana::R, 2).only_for(ManaRestriction::INSTANT_OR_SORCERY_SPELLS), "{T}: Add {R}{R}. Spend this mana only to cast instant and sorcery spells."),
    ],
    ..oracle::TABLET_OF_DISCOVERY
};
