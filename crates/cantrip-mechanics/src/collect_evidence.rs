//! Collect evidence (CR 701.59): "to collect evidence N means to exile any number of cards from your graveyard with total mana
//! value N or greater", a cost, as a spell's additional cost, an activated ability's cost or what a resolving ability asks
//! (`Game::may_pay`). A player with too few cards to reach N can't collect evidence at all (CR 701.59b), so is not asked.

use cantrip_api::*;

/// "As an additional cost to cast this spell, you may collect evidence N" (CR 701.59a, 118.8): exiling cards with total mana value
/// N or more from your graveyard, chosen as the spell is cast; the card reads `paid` for "if evidence was collected". Deadly
/// Cover-Up, Analyze the Pollen, Behind the Mask and Crimestopper Sprite print it.
pub const fn optional(n: u32) -> OptionalCost {
    OptionalCost::new(Cost { collect_evidence: n, ..Cost::NONE })
}

/// "You may collect evidence `N`" as the ability resolves (CR 701.59a, 608.2d, 118.12): its controller is asked whether to exile
/// cards with total mana value `N` or more from their graveyard, and chooses them as they pay. Whether they collected evidence:
/// the "if you do" of the sentence. Surveillance Monitor prints it with 4.
pub fn may_collect_evidence<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<bool, Suspend> {
    g.may_pay(item.controller, item.object, item.object, Cost { collect_evidence: N, ..Cost::NONE }, &COLLECT_EVIDENCE)
}

/// "You may collect evidence X" as the ability resolves (CR 701.59a, 107.3f, 608.2d): its controller chooses X as they pay, in
/// the one question `question` words, raised no further than their graveyard could pay (CR 701.59b), and chooses the cards with
/// total mana value X or more as they pay. X = 0 exiles nothing and is still collecting evidence (the Incinerator of the
/// Guilty ruling). The X collected, or `None` when they declined: "when you do" (CR 603.12) is whether it is `Some`, and a
/// reflexive ability that reads X is given it (`Game::reflexive_trigger_counted`). Incinerator of the Guilty prints it.
pub fn may_collect_evidence_x(g: &mut dyn Game, item: &StackItem, question: &'static Question) -> Result<Option<u32>, Suspend> {
    g.may_pay_x(item.controller, item.object, item.object, Cost { collect_evidence_x: true, ..Cost::NONE }, question)
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
