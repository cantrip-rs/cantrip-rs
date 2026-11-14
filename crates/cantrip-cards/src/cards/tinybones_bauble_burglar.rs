use cantrip_api::*;
use cantrip_mechanics::{effects, your_turn};
use cantrip_oracle as oracle;

/// "Whenever an opponent discards a card" (CR 701.9a, 603.2): each card of a discard is its own event, so the ability triggers once
/// for each card an opponent of the ability's controller discards, and the card, in the graveyard, is what it is about
/// (`StackItem::subject`).
fn opponent_discards_a_card(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Discarded { player, card, .. } = event else { return None };
    if q.state().opponents(me.controller).any(|o| o == *player) { Fired::about(Target::Object(*card)) } else { None }
}

/// "Exile it from their graveyard with a stash counter on it" (CR 701.13, 122.1): the discarded card, if it is the object in its
/// owner's graveyard (a discard a replacement sent elsewhere, Leyline of the Void, madness, never reached it; a card that has left is
/// another object, CR 400.7). The counter is put on the card as it is in exile,
/// by this ability's controller.
fn exile_with_a_stash_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !g.state().zone(Zone::graveyard(g.obj(card).owner)).contains(&card) {
        return Ok(());
    }
    let exiled = g.exile(card);
    g.add_counters_by(item.controller, exiled, CounterKind::Stash, 1);
    Ok(())
}

/// "During your turn, you may play cards you don't own with stash counters on them from exile" (CR 601.3, 604.1): this
/// permanent's controller ("you", CR 109.5) on their own turn, any card in exile with a stash counter that they don't own,
/// whichever Tinybones exiled it (the rulings). A land is played as a land play and a spell is cast at the time its type
/// allows, for all its costs.
fn allows(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    let object = q.obj(card);
    player == asker.controller && your_turn(q, asker) && object.owner != player && object.counters(CounterKind::Stash) > 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DISCARDED, opponent_discards_a_card),
            exile_with_a_stash_counter,
            "Whenever an opponent discards a card, exile it from their graveyard with a stash counter on it.",
        ),
        ability(
            cost("{3}{B}").and(TAP),
            effects::each_opponent_discards_a_card,
            "{3}{B}, {T}: Each opponent discards a card. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    // "... and mana of any type can be spent to cast those spells" (CR 609.4b).
    exile_casts: &[ExileCast {
        text: "Play during your turn, with mana of any type",
        from: ZoneKind::Exile,
        allows,
        additional: Cost::NONE,
        any_type: true,
        any_color: false,
        lands: true,
        life_for_mana_value: false,
        instead: None,
    }],
    ..oracle::TINYBONES_BAUBLE_BURGLAR
};
