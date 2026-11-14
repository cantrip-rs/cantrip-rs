use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Cast this spell only during your declare attackers step." (CR 601.3): the declare attackers step of a turn of the
/// caster's.
fn during_your_declare_attackers_step(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareAttackers && q.state().turn.active == asker.controller
}

/// "This turn, instead of declaring blockers, ..." (CR 614.1a, 509.1, 611.2a): a replacement of each declaration of
/// blockers until the turn ends, whatever becomes of the spell.
fn instead_of_declaring_blockers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.blockers_instead_this_turn(item, 0);
    Ok(())
}

/// "Any number of creatures they control", for one pile: any of the creatures offered, which are those the defending player
/// controls that may go in another pile. A part that asks nothing more, for its words.
fn any_for_the_pile(_: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::any(choice)
}

const TO_PUT_IN_THE_NEXT_PILE: select::Part = select::part(&(any_for_the_pile as SelectFn), 0, "you control to put in the next pile");

/// "Each defending player chooses any number of creatures they control and divides them into a number of piles equal to
/// the number of attacking creatures for whom that player is the defending player. Creatures those players control that
/// can block additional creatures may likewise be put into additional piles. Assign each pile to a different one of those
/// attacking creatures at random. Each creature in a pile that can block the creature that pile is assigned to does so.
/// (Piles can be empty.)" (CR 509.1a, 700.3): the defending player makes the piles one by one, each creature in no more of
/// them than it can block attackers - a pile none can go in is empty, and not asked about; then each pile in turn is given
/// one of the attackers not yet given a pile, at random. The core makes the blocks the restrictions allow
/// (`BlockersInstead`).
fn divide_into_piles(g: &mut dyn Game, asker: &Asker, over: &DefendingCreatures) -> Result<Vec<(ObjectId, ObjectId)>, Suspend> {
    let mut piles: Vec<Vec<ObjectId>> = Vec::with_capacity(over.attackers.len());
    for _ in &over.attackers {
        let room: Vec<ObjectId> = over
            .creatures
            .iter()
            .filter(|&&(c, most)| count_u32(piles.iter().filter(|p| p.contains(&c)).count()) < most)
            .map(|c| c.0)
            .collect();
        let rule = select::cards(select::ANY_NUMBER).with(TO_PUT_IN_THE_NEXT_PILE).called("creature");
        piles.push(choose::select(g, over.defender, asker.source, Purpose::Choose, room, rule)?);
    }
    let mut unassigned = over.attackers.clone();
    let mut blocks = Vec::new();
    for pile in piles {
        let attacker = unassigned.remove(g.choose_at_random(unassigned.len()));
        blocks.extend(pile.into_iter().map(|c| (c, attacker)));
    }
    Ok(blocks)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], instead_of_declaring_blockers),
    cast_only_if: Some(during_your_declare_attackers_step),
    blockers_instead: &[BlockersInstead {
        text: "This turn, instead of declaring blockers, each defending player divides creatures they control into piles, one for each creature attacking them, and each pile is assigned to one of those creatures at random.",
        blocks: divide_into_piles,
    }],
    ..oracle::CAMOUFLAGE
};
