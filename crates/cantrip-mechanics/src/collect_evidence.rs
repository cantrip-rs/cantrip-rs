//! Collect evidence (CR 701.59): "to collect evidence N means to exile any number of cards from your graveyard with total mana
//! value N or greater", a cost, as a spell's additional cost, an activated ability's cost or what a resolving ability asks
//! (`Game::may_pay`). A player with too few cards to reach N can't collect evidence at all (CR 701.59b), so is not asked.

use cantrip_api::*;

/// "As an additional cost to cast this spell, you may collect evidence N" (CR 701.59a, 118.8): exiling cards with total mana value
/// N or more from your graveyard, chosen as the spell is cast; the card reads `paid` for "if evidence was collected". Deadly
/// Cover-Up, Analyze the Pollen, Behind the Mask and Crimestopper Sprite print it.
pub const fn optional(n: u8) -> OptionalCost {
    OptionalCost::new(Cost { collect_evidence: n, ..Cost::NONE })
}

/// "You may collect evidence `N`" as the ability resolves (CR 701.59a, 608.2d, 118.12): its controller is asked whether to exile
/// cards with total mana value `N` or more from their graveyard, and chooses them as they pay. Whether they collected evidence:
/// the "if you do" of the sentence. Surveillance Monitor prints it with 4.
pub fn may_collect_evidence<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<bool, Suspend> {
    g.may_pay(item.controller, item.object, item.object, Cost { collect_evidence: N, ..Cost::NONE }, &COLLECT_EVIDENCE)
}

const COLLECT_EVIDENCE: Question = Question::new("CollectEvidence", "collect evidence?", "", "Collect evidence", "Don't collect evidence");

/// "Whenever you collect evidence" (CR 701.59a): its controller paid a cost to collect evidence, as a cost or as an effect asked
/// (`Event::CollectedEvidence`). Each cost paid triggers it once, for any reason, not only the ability that asked.
pub fn you_collected_evidence(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::CollectedEvidence { player, .. } = *event else { return None };
    (player == me.controller).then_some(Fired { subject: Target::Player(player), data: [0; 2] })
}

/// "Whenever you collect evidence".
pub const YOU_COLLECT_EVIDENCE: Trigger = when_asked(EventMask::COLLECTED_EVIDENCE, you_collected_evidence);
