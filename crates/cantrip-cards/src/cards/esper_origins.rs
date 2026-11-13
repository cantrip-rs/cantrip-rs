use cantrip_api::*;
use cantrip_mechanics::{effects, flashback, targets, top_of_library};
use cantrip_oracle as oracle;

/// "Surveil 2. You gain 2 life. If this spell was cast from a graveyard, exile it, then put it onto the battlefield
/// transformed under its owner's control with a finality counter on it."
///
/// Surveil (CR 701.25) and the life (CR 119.3) are the caster's. "Cast from a graveyard" is the zone the spell was cast from
/// (CR 601.2a), by flashback or any other permission; a copy of the spell was never cast (CR 707.10) and does nothing more.
/// The card leaves the stack for exile as the spell's own instruction, before flashback's "exile it instead" could apply
/// (CR 702.34a, 608.2n), and then enters with its back face up, Summon: Esper Maduin, as a new object (CR 712.14a, 400.7)
/// under its owner's control, a Saga. It enters with the finality counter on it, put by its controller (CR 122.6, 122.6a,
/// 614.1c), beside the lore counter every Saga enters with (CR 714.3a); a replacement that keeps it from entering leaves
/// both off. The finality counter makes it go to exile rather than to a graveyard from the battlefield (CR 122.1h). A
/// replacement that kept the card out of exile leaves it where it went.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)?;
    g.gain_life(item.controller, 2);
    if g.obj(item.object).cast_from != Some(ZoneKind::Graveyard) {
        return Ok(());
    }
    let exiled = g.exile(item.object);
    if g.obj(exiled).zone.is(ZoneKind::Exile) {
        let owner = g.obj(exiled).owner;
        g.put_onto_battlefield_transformed_with_counters(exiled, owner, false, &[(CounterKind::Finality, 1)])?;
    }
    Ok(())
}

// --- Summon: Esper Maduin --------------------------------------------------------------------------------------------

/// I: "Reveal the top card of your library. If it's a permanent card, put it into your hand." (CR 701.20) A card that is
/// not a permanent card stays on top, revealed; an empty library reveals nothing.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&top) = top_of_library(g, item.controller, 1).first() else { return Ok(()) };
    g.reveal(item.controller, &[top]);
    if targets::is_permanent_card(&g.chars(top)) {
        g.return_to_hand_revealed(&[top]);
    }
    Ok(())
}

/// II: "Add {G}{G}." A chapter ability is a triggered ability, not a mana ability: it uses the stack (CR 605.1b, the
/// ruling).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::G, 2);
    Ok(())
}

/// III: "Other creatures you control get +2/+2 and gain trample until end of turn." The creatures are those its controller
/// controls as it resolves, not the Saga itself, and none that enters later (CR 611.2c).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_other_creatures_you_control_and_gain(g, item, 2, 2, &[Keyword::Trample]);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Reveal the top card of your library. If it's a permanent card, put it into your hand."),
        chapter(2, chapter_two, "II \u{2014} Add {G}{G}.").produces_mana(),
        chapter(3, chapter_three, "III \u{2014} Other creatures you control get +2/+2 and gain trample until end of turn."),
    ],
    ..oracle::SUMMON_ESPER_MADUIN
};

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{3}{G}"), "Flashback {3}{G}")],
    back: Some(&BACK),
    ..oracle::ESPER_ORIGINS
};
