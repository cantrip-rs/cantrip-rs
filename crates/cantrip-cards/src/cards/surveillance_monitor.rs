use cantrip_api::*;
use cantrip_mechanics::{collect_evidence, tokens, triggers};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// "When this creature enters, you may collect evidence 4." (CR 701.59a, 603.5): its controller chooses whether to pay by exiling
/// cards with total mana value 4 or more from their graveyard; the Thopter is the card's other ability, which that cost triggers.
fn may_collect_evidence_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    collect_evidence::may_collect_evidence::<4>(g, item)?;
    Ok(())
}

/// "Whenever you collect evidence, create a 1/1 colorless Thopter artifact creature token with flying." (CR 701.59a, 603.2, 111.1):
/// for any evidence you collect, not only this creature's own.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            may_collect_evidence_four,
            "When this creature enters, you may collect evidence 4. (Exile cards with total mana value 4 or greater from your graveyard.)",
        ),
        triggered(
            collect_evidence::YOU_COLLECT_EVIDENCE,
            tokens::create_a_thopter,
            "Whenever you collect evidence, create a 1/1 colorless Thopter artifact creature token with flying.",
        ),
    ],
    ..oracle::SURVEILLANCE_MONITOR
};

pub static EXTRA: &[&CardDef] = &[&THOPTER];
