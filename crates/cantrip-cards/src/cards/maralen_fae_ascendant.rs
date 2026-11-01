use cantrip_api::*;
use cantrip_mechanics::{history, targets, top_of_library};
use cantrip_oracle as oracle;

/// Whether `id` is an Elf or a Faerie (CR 205.3m), the creature types read through the words of Maralen's text (CR 612).
fn elf_or_faerie(q: &dyn Query, source: ObjectId, id: ObjectId) -> bool {
    let words = q.words(source);
    let chars = q.chars(id);
    chars.has_subtype(words.subtype(Subtype::Elf)) || chars.has_subtype(words.subtype(Subtype::Faerie))
}

/// "Whenever Maralen or another Elf or Faerie you control enters" (CR 603.6a, 201.5): Maralen herself, whatever she is as she
/// enters, or another permanent under your control as it entered that is an Elf, a Faerie or both - one event, so one trigger.
fn maralen_or_another_elf_or_faerie_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && (*new == me.source || (q.controller(*new) == me.controller && elf_or_faerie(q, me.source, *new))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Exile the top two cards of target opponent's library." They are exiled with Maralen (CR 607.2a), which the second ability
/// reads: linked to the permanent that triggered, so a Maralen that left and came back is a new object that finds none of them
/// (CR 400.7). An opponent with fewer than two cards loses what they have (CR 701.13).
fn exile_top_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        let top = top_of_library(g, opponent, 2);
        g.exile_with(&top, g.linked(item), false);
    }
    Ok(())
}

/// "Once each turn, you may cast a spell with mana value less than or equal to the number of Elves and Faeries you control from
/// among cards exiled with Maralen this turn without paying its mana cost": its controller ("you", CR 109.5), a card Maralen
/// exiled (CR 607.2a) that the exile move of this turn made (CR 400.7), judged as the spell it would be (its mana value, CR
/// 202.3), against the permanents under their control that are an Elf or a Faerie, one that is both counted once. It is used up
/// by a cast by it this turn, which the history records as made by this permanent (`Cast::granted_by`), on any player's turn.
fn once_each_turn(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    let elves_and_faeries = q
        .state()
        .battlefield()
        .iter()
        .filter(|&&id| q.controller(id) == player && elf_or_faerie(q, asker.source, id))
        .count();
    player == asker.controller
        && q.exiled_with(q.linked_to(asker)).contains(&card)
        && history::arrived_this_turn(q, card)
        && q.mana_value(card) as usize <= elves_and_faeries
        && !history::permission_used_this_turn(q, asker.source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, maralen_or_another_elf_or_faerie_enters),
        exile_top_two,
        "Whenever Maralen or another Elf or Faerie you control enters, exile the top two cards of target opponent's library.",
    )
    .targets(&[targets::OPPONENT])],
    exile_casts: &[ExileCast {
        text: "Cast without paying its mana cost",
        allows: once_each_turn,
        additional: Cost::NONE,
        any_type: false,
        lands: false,
        life_for_mana_value: false,
        instead: Some(ManaCost::ZERO),
    }],
    ..oracle::MARALEN_FAE_ASCENDANT
};
