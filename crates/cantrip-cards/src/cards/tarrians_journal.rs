use cantrip_api::*;
use cantrip_mechanics::{cast_from_your_graveyard, effects, replacements, targets};
use cantrip_oracle as oracle;

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}
/// "{T}: You may cast a creature spell from your graveyard this turn" (CR 611.2a): the ability gives its controller a way to
/// cast one, for the turn (`CardDef::lasting_ways`, `once`), that does not end if the Tomb does. It chooses no card: the
/// permission is for a creature spell cast later (the ruling), and each activation gives one cast.
fn you_may_cast_a_creature_spell_from_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.give_a_way_this_turn(item, item.controller, 0, true);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(
            TAP,
            you_may_cast_a_creature_spell_from_your_graveyard,
            "{T}: You may cast a creature spell from your graveyard this turn. If you do, it enters with a finality counter on it and is a Vampire in addition to its other types.",
        ),
    ],
    lasting_ways: &[GrantedWay {
        text: "You may cast a creature spell from your graveyard this turn.",
        cost: cast_from_your_graveyard::card_of_type::<{ CardType::Creature as u8 }>,
        permission: true,
        spell_replacement: Some(0),
        // "It ... is a Vampire in addition to its other types" (CR 205.1b): on the stack and as the creature it becomes.
        spell_effects: &[EffectKind::AddSubtype(Subtype::Vampire)],
        ..GrantedWay::CAST
    }],
    // "If you do, it enters with a finality counter on it": a replacement effect about the spell cast by the way
    // (`GrantedWay::spell_replacement`), made as it is cast, so it applies though the Tomb has left before it resolves.
    granted_replacements: &[replacements::SPELL_ENTERS_WITH_FINALITY
        .text("If you do, it enters with a finality counter on it.")],
    ..oracle::THE_TOMB_OF_ACLAZOTZ
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, targets::filters::another_creature_or_artifact)),
            draw_a_card,
            "{T}, Sacrifice another artifact or creature: Draw a card. Activate only as a sorcery.",
        )
        .sorcery_speed(),
        ability(cost("{2}").and(TAP).and(DISCARD_HAND), effects::transform_self, "{2}, {T}, Discard your hand: Transform Tarrian's Journal."),
    ],
    back: Some(&BACK),
    ..oracle::TARRIANS_JOURNAL
};
