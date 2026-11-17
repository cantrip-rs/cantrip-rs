use cantrip_api::*;
use cantrip_mechanics::{choose, delayed, saddle, select, triggers};
use cantrip_oracle as oracle;

/// The creatures that saddled `mount` this turn (CR 702.171c, `Event::Saddled`) and are nonlegendary creatures on the
/// battlefield now, as the objects that saddled it: one that left and came back is a new object (CR 400.7) and is not one of them.
fn nonlegendary_saddlers(q: &dyn Query, mount: ObjectId) -> Vec<ObjectId> {
    let mut found: Vec<ObjectId> = Vec::new();
    for record in q.history().this_turn() {
        if let Event::Saddled { creature, mount: saddled } = record.event
            && saddled == mount
            && !found.contains(&creature)
            && q.state().battlefield().contains(&creature)
            && q.chars(creature).is(CardType::Creature)
            && !q.chars(creature).supertypes.legendary
        {
            found.push(creature);
        }
    }
    found
}

/// "Choose a nonlegendary creature that saddled it this turn and create a tapped and attacking token that's a copy of it.
/// Sacrifice that token at the beginning of the next end step. Repeat this process once." (CR 702.171c, 707.2, 508.4, 603.7):
/// the whole process happens twice, the second choice made afresh (the same creature or another, the rulings), each token
/// attacking what its controller chooses as it enters. The tokens were never declared as attackers, so nothing that watches
/// attackers being declared sees them. Nothing happens when no such creature is left to choose.
fn copy_a_saddler(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..2 {
        let candidates = nonlegendary_saddlers(g, item.object);
        let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, candidates, select::cards(select::exactly(1)))?;
        if let Some(&creature) = chosen.first() {
            let values = g.copiable_values(creature);
            let tokens = g.create_token_copies_attacking(values, item.controller, 1, true, None)?;
            g.end_sentence();
            if !tokens.is_empty() {
                delayed::at_next_end_step_about(g, item, 0, &tokens);
            }
        }
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            copy_a_saddler,
            "Whenever this creature attacks while saddled, choose a nonlegendary creature that saddled it this turn and create a tapped and attacking token that's a copy of it. Sacrifice that token at the beginning of the next end step. Repeat this process once.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice that token at the beginning of the next end step.")],
    ..oracle::CALAMITY_GALLOPING_INFERNO
};
