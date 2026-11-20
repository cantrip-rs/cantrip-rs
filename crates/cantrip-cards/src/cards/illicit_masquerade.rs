use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Up to one other target creature card from your graveyard" (CR 115.1, 601.2c, 730.3c): "other" than every card the dying
/// creature became (the trigger's subject), which are in the graveyard as the target is chosen.
fn other_creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    let Target::Object(old) = q.event_subject() else { return false };
    !q.found_all_after_move(old).contains(&id) && (targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.filter)(q, asker, t)
}

/// "Exile it. Return up to one other target creature card from your graveyard to the battlefield" (CR 608.2c, 400.7, 701.13a):
/// "it" is each card the creature became in the graveyard (CR 730.3c), exiled only if still there; the creature card returned
/// was chosen as the ability went on the stack and is never that card.
fn exile_it_then_return_another_creature_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(old) = item.subject {
        effects::exile_what_it_became(g, old);
    }
    g.end_sentence();
    effects::return_first_target_to_battlefield(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::counter_on_each_creature_you_control::<{ CounterKind::Impostor as usize }>,
            "When this enchantment enters, put an impostor counter on each creature you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::creature_you_control_with_counter_dies::<{ CounterKind::Impostor as usize }>),
            exile_it_then_return_another_creature_card,
            "Whenever a creature you control with an impostor counter on it dies, exile it. Return up to one other target creature card from your graveyard to the battlefield.",
        )
        .targets(&[target(other_creature_card_in_your_graveyard).optional()]),
    ],
    ..oracle::ILLICIT_MASQUERADE
};
