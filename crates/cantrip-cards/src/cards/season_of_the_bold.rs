use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL;
use cantrip_mechanics::{delayed, effects, library, tokens};
use cantrip_oracle as oracle;

/// {P} - "Create a tapped Treasure token." (CR 111.10a).
fn tapped_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_tapped_treasure(g, item.controller)
}

/// {P}{P} - "Exile the top two cards of your library. Until the end of your next turn, you may play them." (CR 701.18b, 611.2a)
fn exile_top_two_play_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_until_end_of_next_turn(g, item.controller, 2);
    Ok(())
}

/// {P}{P}{P} - "Until the end of your next turn, whenever you cast a spell, Season of the Bold deals 2 damage to up to one
/// target creature." A delayed triggered ability (CR 603.7b), made as this mode resolves and lasting through its
/// controller's next turn; each instance of the mode makes its own.
fn burn_on_each_spell_through_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::through_your_next_turn(g, item, 0, item.object);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(tapped_treasure, "{P} \u{2014} Create a tapped Treasure token.").pawprints(1),
    mode(
        exile_top_two_play_them,
        "{P}{P} \u{2014} Exile the top two cards of your library. Until the end of your next turn, you may play them.",
    )
    .pawprints(2),
    mode(
        burn_on_each_spell_through_next_turn,
        "{P}{P}{P} \u{2014} Until the end of your next turn, whenever you cast a spell, Season of the Bold deals 2 damage to up to one target creature.",
    )
    .pawprints(3),
])
.pawprints(5);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODES),
    delayed: &[triggered(
        YOU_CAST_A_SPELL,
        // "Season of the Bold deals 2 damage to up to one target creature" (CR 120.3): the target chosen as the delayed ability
        // triggers (CR 603.3d), none at all too; the source is the card as it last existed (CR 113.7a).
        effects::damage_first_target::<2>,
        "Whenever you cast a spell, Season of the Bold deals 2 damage to up to one target creature.",
    )
    .targets(&[up_to_one(cantrip_mechanics::targets::creature_filter)])],
    ..oracle::SEASON_OF_THE_BOLD
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
