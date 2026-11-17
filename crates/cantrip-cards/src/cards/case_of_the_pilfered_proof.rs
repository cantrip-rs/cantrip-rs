use cantrip_api::*;
use cantrip_mechanics::{lands, replacements::tokens_under_your_control, solved, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever a Detective you control enters or is turned face up" (CR 603.6a, 708.8, 205.3): a Detective under your control as it
/// enters (a face-down permanent is a 2/2 with no subtypes, CR 708.2, so it enters as no Detective), and a permanent you control that
/// is a Detective once it is face up, whoever turned it. The trigger is about that permanent (`StackItem::subject`), the type read
/// through the words of the Case's text (CR 612).
fn detective_you_control_enters_or_is_turned_face_up(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Detective) }>(q, me, event)
        .or_else(|| triggers::of_subtype_you_control_turned_face_up::<{ lands::sub(Subtype::Detective) }>(q, me, event))
}

/// "Put a +1/+1 counter on it" (CR 122.1): the permanent that entered or was turned face up, the trigger's subject. If it has left the
/// battlefield by now it is a new object (CR 400.7) and takes nothing (CR 608.2b).
fn counter_on_that_detective(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(detective) = item.subject.object().filter(|&id| targets::permanent(g, id)) else { return Ok(()) };
    g.add_counters(detective, CounterKind::PlusOne, 1);
    Ok(())
}

/// "To solve — You control three or more Detectives" (CR 719.3a): an intervening "if" (CR 603.4), asked as the end step begins and as
/// the ability resolves. The type is read through the words of the Case's text (CR 612).
fn three_or_more_detectives(q: &dyn Query, asker: &Asker) -> bool {
    let detective = lands::word(q, asker.source, lands::sub(Subtype::Detective));
    q.permanents_of(asker.controller).into_iter().filter(|&id| q.chars(id).has_subtype(detective)).count() >= 3
}

/// "Solved — If one or more tokens would be created under your control": as long as this Case is solved (CR 702.169b), tokens that
/// would be yours, whoever makes them (ruling 2024-02-02).
fn tokens_under_your_control_while_solved(q: &dyn Query, me: &Asker, proposal: &Proposal) -> bool {
    solved::is_solved(q, me.source) && tokens_under_your_control(q, me, proposal)
}

/// "... those tokens plus a Clue token are created instead" (CR 614.1a): one Clue more in the same event, however many kinds of token it
/// makes (`TokenCreation::plus`).
fn plus_a_clue(q: &dyn Query, me: &Asker, p: &mut Proposal) {
    if let Proposal::TokenCreation(t) = p {
        let clue = q.card_id(&tokens::CLUE);
        t.plus = Some((clue, q.token_copy(clue, me.source)));
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::TURNED_FACE_UP.0), detective_you_control_enters_or_is_turned_face_up),
            counter_on_that_detective,
            "Whenever a Detective you control enters or is turned face up, put a +1/+1 counter on it.",
        ),
        solved::to_solve(three_or_more_detectives, "To solve \u{2014} You control three or more Detectives. (If unsolved, solve at the beginning of your end step.)"),
    ],
    replacements: &[replacement(
        ReplacementKind::TokenCreation,
        tokens_under_your_control_while_solved,
        plus_a_clue,
        "creates those tokens plus a Clue token instead",
    )],
    ..oracle::CASE_OF_THE_PILFERED_PROOF
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
