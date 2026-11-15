use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

/// The two labels, each the offset of its rules (`RULES`) from `PILE` and from `CHOSEN`.
const LEFT: u8 = 0;
const RIGHT: u8 = 1;
/// The rules' indexes: the two "creatures in a pile with this label" rules, the two "chosen label" rules, then the mark a
/// resolution puts on each attacking creature it labels before anything else.
const PILE: u8 = 0;
const CHOSEN: u8 = 2;
const LABELLED: u8 = 4;

/// "Creatures in a pile with the chosen label" (CR 700.3): the creature this is on was put in that resolution's pile of this
/// label, and stays in it for the rest of the combat whatever it becomes, until it leaves (CR 400.7): the set the words name
/// is the one made as the ability resolved, which nothing entering later joins (the rulings). It forbids no block itself:
/// the attackers' rules (`except_by_flying_or_pile`) read it. So does the `LABELLED` mark.
fn forbids_nothing(_: &dyn Query, _: ObjectId, _: ObjectId, _: ObjectId) -> bool {
    true
}

/// A Raging River rule on `on` (`RULES`): the effect, and the index it names.
fn river_rules<'a>(q: &'a dyn Query, on: ObjectId) -> impl Iterator<Item = (&'a ContinuousEffect, CardId, u8)> + 'a {
    q.state().effects.iter().filter_map(move |e| match e.kind {
        EffectKind::BlockRestriction { card, index } if e.affected == on && std::ptr::eq(q.card(card).lasting_block_rules, &RULES[..]) => {
            Some((e, card, index))
        }
        _ => None,
    })
}

/// "That creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label"
/// (CR 509.1b): asked of every block of `on`, the attacking creature the label was chosen for, as blockers are declared. A
/// blocker with flying then may; any other only if the same resolution put it in a pile with the label chosen for `on` - not
/// a creature that entered, became a creature or had flying as the piles were made (the rulings). Each resolution's
/// restriction holds on its own, so a creature labelled by two of them answers to both.
///
/// Which piles are the label's own resolution's is read from the effects' timestamps: each effect begins with a new, later
/// one, and a resolution makes all of its effects in one instruction - its `LABELLED` marks, then its piles, then its labels -
/// so the piles made between `on`'s latest mark from that source before the label and the label itself are that
/// resolution's, and no other's. The effects on `on` end together, as it leaves or as the combat ends.
fn except_by_flying_or_pile(q: &dyn Query, on: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    if attacker != on || q.chars(blocker).has(Keyword::Flying) {
        return true;
    }
    river_rules(q, on).filter(|&(_, _, index)| (CHOSEN..LABELLED).contains(&index)).all(|(chosen, card, index)| {
        let labelled = EffectKind::BlockRestriction { card, index: LABELLED };
        let start = river_rules(q, on)
            .filter(|(e, ..)| e.source == chosen.source && e.kind == labelled && e.timestamp < chosen.timestamp)
            .map(|(e, ..)| e.timestamp)
            .max();
        let pile = EffectKind::BlockRestriction { card, index: index - CHOSEN + PILE };
        river_rules(q, blocker).any(|(e, ..)| {
            e.source == chosen.source && e.kind == pile && start < Some(e.timestamp) && e.timestamp < chosen.timestamp
        })
    })
}

/// The rules, in index order (`PILE`, `CHOSEN`, `LABELLED`).
static RULES: [LastingBlockRule; 5] = [
    LastingBlockRule { text: "In the \"left\" pile", allows: forbids_nothing },
    LastingBlockRule { text: "In the \"right\" pile", allows: forbids_nothing },
    LastingBlockRule {
        text: "Can't be blocked except by creatures with flying and creatures in a \"left\" pile",
        allows: except_by_flying_or_pile,
    },
    LastingBlockRule {
        text: "Can't be blocked except by creatures with flying and creatures in a \"right\" pile",
        allows: except_by_flying_or_pile,
    },
    LastingBlockRule { text: "Labelled by Raging River", allows: forbids_nothing },
];

/// Any of the creatures offered: the words say what choosing them does.
fn any_of_them(_: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::any(choice)
}

const TO_THE_LEFT_PILE: select::Part =
    select::part(&(any_of_them as SelectFn), 0, "to put in the \"left\" pile (the rest go in the \"right\" pile)");
const TO_LABEL_LEFT: select::Part = select::part(&(any_of_them as SelectFn), 0, "to label \"left\" (the rest are labelled \"right\")");

/// "Each defending player divides all creatures without flying they control into a "left" pile and a "right" pile. Then, for
/// each attacking creature you control, choose "left" or "right." That creature can't be blocked this combat except by creatures
/// with flying and creatures in a pile with the chosen label." Every opponent is a defending player (CR 506.2, 802.2); each
/// divides in APNAP order (CR 101.4), choosing their "left" pile, the rest being the "right" one, either possibly empty (CR
/// 700.3a, 700.3d). Then the ability's controller, seeing the piles, chooses the attacking creatures they control labelled
/// "left", the rest "right" - those attacking as it resolves, not one put onto the battlefield attacking later (the rulings).
/// The restriction begins on them all at once (CR 611.2a), until the combat ends; the piles are kept on their creatures for
/// it to read, in the order `except_by_flying_or_pile` reads them by.
fn divide_and_choose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let defenders: Vec<PlayerId> = g.state().apnap().filter(|&p| p != you).collect();
    let mut piles: [Vec<ObjectId>; 2] = [Vec::new(), Vec::new()];
    for defender in defenders {
        let creatures: Vec<ObjectId> = g
            .permanents_of(defender)
            .into_iter()
            .filter(|&id| g.chars(id).is(CardType::Creature) && !g.chars(id).has(Keyword::Flying))
            .collect();
        let rule = select::cards(select::ANY_NUMBER).with(TO_THE_LEFT_PILE).called("creature");
        let left = choose::select(g, defender, item.object, Purpose::Choose, creatures.clone(), rule)?;
        piles[RIGHT as usize].extend(creatures.into_iter().filter(|c| !left.contains(c)));
        piles[LEFT as usize].extend(left);
    }
    let attackers: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.is_attacking(id) && g.controller(id) == you && g.chars(id).is(CardType::Creature))
        .collect();
    let rule = select::cards(select::ANY_NUMBER).with(TO_LABEL_LEFT).called("attacking creature");
    let left = choose::select(g, you, item.object, Purpose::Choose, attackers.clone(), rule)?;
    let card = item.ability.card;
    let restrict = |g: &mut dyn Game, creature: ObjectId, index: u8| {
        g.add_effect(item.object, creature, EffectKind::BlockRestriction { card, index }, Duration::EndOfCombat);
    };
    instruction(g, |g| {
        for &attacker in &attackers {
            restrict(g, attacker, LABELLED);
        }
        for label in [LEFT, RIGHT] {
            for &creature in &piles[label as usize] {
                restrict(g, creature, PILE + label);
            }
        }
        for &attacker in &attackers {
            restrict(g, attacker, CHOSEN + if left.contains(&attacker) { LEFT } else { RIGHT });
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        divide_and_choose,
        "Whenever one or more creatures you control attack, each defending player divides all creatures without flying they control into a \"left\" pile and a \"right\" pile. Then, for each attacking creature you control, choose \"left\" or \"right.\" That creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label.",
    )],
    lasting_block_rules: &RULES,
    ..oracle::RAGING_RIVER
};
