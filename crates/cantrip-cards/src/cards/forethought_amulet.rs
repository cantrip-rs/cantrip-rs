use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this artifact unless you pay {3}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_artifact_unless_pay_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{3}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "If an instant or sorcery source would deal 3 or more damage to you, it deals 2 damage to you instead." (CR 604.1).
fn if_instant_sorcery_source_would_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::Damage(e) = proposal else { return false };
    let (source, target, amount) = (e.source, &e.target, e.amount);
    if (q.chars(source).is(CardType::Instant) || q.chars(source).is(CardType::Sorcery))
        && matches!(* target, Target::Player(p) if asker.controller == p)
        && (i64::from(amount)) >= 3
    {
        return true;
    }
    false
}

/// "If an instant or sorcery source would deal 3 or more damage to you, it deals 2 damage to you instead." (CR 604.1).
fn if_instant_sorcery_source_would_apply(
    _: &dyn Query,
    _: &Asker,
    proposal: &mut Proposal,
) {
    let seen = *proposal;
    let Proposal::Damage(_) = &seen else { return };
    let Proposal::Damage(e) = proposal else { return };
    e.amount = 2;
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_artifact_unless_pay_3,
            "At the beginning of your upkeep, sacrifice this artifact unless you pay {3}.",
        ),
    ],
    replacements: &[
        replacement(
            ReplacementKind::Damage,
            if_instant_sorcery_source_would_applies,
            if_instant_sorcery_source_would_apply,
            "If an instant or sorcery source would deal 3 or more damage to you, it deals 2 damage to you instead.",
        ),
    ],
    ..oracle::FORETHOUGHT_AMULET
};
