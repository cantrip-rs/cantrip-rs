use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Permanent spells you cast that have an Adventure cost {1} less to cast" (CR 601.2f, 118.7a): a permanent spell as it is cast, so the
/// creature half of an adventurer card and not the Adventure it is cast as (an instant or sorcery spell, CR 715.3b); generic mana only.
fn permanent_spells_with_an_adventure_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && targets::is_permanent_card(&q.chars(spell)) && library::card_has_an_adventure(q, spell) {
        CostDelta::less(1)
    } else {
        CostDelta::NONE
    }
}

/// The Adventure (CR 715): "Mill seven cards. Then put all cards that have an Adventure from among the milled cards into your hand."
/// The milled cards are found where they went, and each has an Adventure whatever it is in the graveyard (the ruling).
fn seek_thrills(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let milled = g.mill(item.controller, 7);
    g.end_sentence();
    let adventurers: Vec<ObjectId> = milled.into_iter().filter(|&card| library::card_has_an_adventure(g, card)).collect();
    g.return_to_hand(&adventurers);
    Ok(())
}

pub static SEEK_THRILLS: CardDef = CardDef { spell: spell(&[], seek_thrills), ..oracle::SEEK_THRILLS };

pub static CARD: CardDef = CardDef {
    cost_changes: &[permanent_spells_with_an_adventure_cost_one_less],
    adventure: Some(&SEEK_THRILLS),
    ..oracle::BELUNA_GRANDSQUALL
};

pub static EXTRA: &[&CardDef] = &[&SEEK_THRILLS];
