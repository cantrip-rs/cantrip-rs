use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL;
use cantrip_mechanics::{effects, exile_and_return_transformed, tokens};
use cantrip_oracle as oracle;

/// III: "Exile this Saga, then return it to the battlefield transformed under your control": the ability's controller's, who
/// need not own it (CR 712.14a). A Saga that is not a double-faced card, a copy of one, stays in exile (the ruling).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

// --- Avatar Kuruk ----------------------------------------------------------------------------------------------------

/// "Whenever you cast a spell, create a 1/1 colorless Spirit creature token with 'This token can't block or be blocked by
/// non-Spirit creatures.'" (CR 601.2i, 603.2): any spell its controller casts, the token made as the ability resolves.
fn create_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT, 1)?;
    Ok(())
}

/// "Take an extra turn after this one" (CR 500.7): the extra turn is the controller's, and comes before any created earlier
/// (the most recently created is taken first).
fn extra_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_extra_turns(item.controller, 1);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(YOU_CAST_A_SPELL, create_spirit, "Whenever you cast a spell, create a 1/1 colorless Spirit creature token with \"This token can't block or be blocked by non-Spirit creatures.\""),
        ability(cost("{20}"), extra_turn, "Exhaust \u{2014} Waterbend {20}: Take an extra turn after this one. (Activate each exhaust ability only once.)")
            .waterbend()
            .exhaust(),
    ],
    ..oracle::AVATAR_KURUK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::scry_then_draw::<2>, "I \u{2014} Scry 2, then draw a card."),
        chapter(2, effects::scry_then_draw::<2>, "II \u{2014} Scry 2, then draw a card."),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    back: Some(&BACK),
    ..oracle::THE_LEGEND_OF_KURUK
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT];
