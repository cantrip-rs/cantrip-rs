use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{collect_evidence, tokens};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a)
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// "You may collect evidence 4." (CR 701.59a, 603.5): asked as the ability resolves, and not at all with less than 4 mana value in the
/// graveyard (CR 701.59b); the Clue is the card's other ability, which that cost triggers.
fn may_collect_evidence_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    collect_evidence::may_collect_evidence::<4>(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            may_collect_evidence_four,
            "At the beginning of combat on your turn, you may collect evidence 4. (Exile cards with total mana value 4 or greater from your graveyard.)",
        ),
        triggered(collect_evidence::YOU_COLLECT_EVIDENCE, investigate, "Whenever you collect evidence, investigate."),
    ],
    ..oracle::EVIDENCE_EXAMINER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
