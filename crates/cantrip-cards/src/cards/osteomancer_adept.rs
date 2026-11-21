use cantrip_api::*;
use cantrip_mechanics::{cast_from_your_graveyard, forage, replacements};
use cantrip_oracle as oracle;

/// "{T}: Until end of turn, you may cast creature spells from your graveyard by foraging in addition to paying their other costs"
/// (CR 611.2a, 118.8): the ability gives its controller a way to cast, for the turn, that does not end if the Adept does
/// (`CardDef::lasting_ways`).
fn you_may_cast_creatures_from_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.give_a_way_this_turn(item, item.controller, 0, false);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        you_may_cast_creatures_from_your_graveyard,
        "{T}: Until end of turn, you may cast creature spells from your graveyard by foraging in addition to paying their other costs. If you cast a spell this way, that creature enters with a finality counter on it.",
    )],
    lasting_ways: &[GrantedWay {
        text: "Until end of turn, you may cast creature spells from your graveyard by foraging in addition to paying their other costs.",
        cost: cast_from_your_graveyard::card_of_type::<{ CardType::Creature as u8 }>,
        permission: true,
        additional_choice: &[forage::EXILE_THREE_CARDS, forage::SACRIFICE_A_FOOD],
        spell_replacement: Some(0),
        ..GrantedWay::CAST
    }],
    // "If you cast a spell this way, that creature enters with a finality counter on it": a replacement effect about the spell
    // cast by the way (`GrantedWay::spell_replacement`), made as it is cast, so it applies though the Adept is gone.
    granted_replacements: &[replacements::SPELL_ENTERS_WITH_FINALITY
        .text("If you cast a spell this way, that creature enters with a finality counter on it.")],
    ..oracle::OSTEOMANCER_ADEPT
};
