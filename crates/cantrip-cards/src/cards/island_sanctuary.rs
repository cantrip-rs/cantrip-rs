use cantrip_api::*;
use cantrip_mechanics::replacements::instead_of_drawing;
use cantrip_oracle as oracle;

static INSTEAD_0: Question = Question::new(
    "Instead",
    "Skip this draw to restrict which creatures can attack you until your next turn?",
    "Draw the card as normal.",
    "Skip the draw",
    "Draw",
);

/// "If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk." (CR 614.1a).
fn if_would_draw_card_during_applies(
    q: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::Draw(d) = proposal else { return false };
    if q.state().turn.step() == Step::Draw && q.state().turn.active == d.player
        && asker.controller == d.player
    {
        return true;
    }
    false
}

/// "If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk." (CR 508.1c).
fn permitted_attackers_0(
    q: &dyn Query,
    asker: &Asker,
    _: Target,
    object: ObjectId,
) -> bool {
    q.chars(object).is(CardType::Creature)
        && !(q.chars(object).keywords.has(Keyword::Flying)
            || Keyword::landwalk(q.words(asker.source).subtype(Subtype::Island))
                .is_some_and(|walk| q.chars(object).keywords.matches(walk)))
}

/// "If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk." (CR 614.1a, 611.2b).
fn if_would_draw_card_during_then(
    g: &mut dyn Game,
    asker: &Asker,
    _: &Proposal,
    _: &Proposal,
) {
    instruction(
        g,
        |g| {
            let rule_card = g.card_id(&CARD);
            g.add_player_effect(
                asker.controller,
                asker.source,
                EffectKind::LastingRule {
                    card: rule_card,
                    index: 0,
                    about: Target::Player(asker.controller),
                },
                Duration::UntilNextTurnOf(asker.controller),
            );
        },
    );
}

pub static CARD: CardDef = CardDef {
    replacements: &[
        replacement(
                ReplacementKind::Draw,
                if_would_draw_card_during_applies,
                instead_of_drawing,
                "If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.",
            )
            .then(if_would_draw_card_during_then)
            .optional(&INSTEAD_0),
    ],
    lasting_rules: &[
        LastingRule {
            text: "If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.",
            rule: LastingRuleKind::CantAttackYou,
            applies: permitted_attackers_0,
        },
    ],
    ..oracle::ISLAND_SANCTUARY
};
