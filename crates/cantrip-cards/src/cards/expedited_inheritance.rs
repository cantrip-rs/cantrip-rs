use cantrip_api::*;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileForDamage", "exile cards from the top of your library?", "Exile that many cards; you may play them until the end of your next turn", "Exile them", "Don't").no_default();

/// "Whenever a creature is dealt damage" (CR 120.2, 603.2c): any creature, whoever controls it or the Enchantment, the damage it is
/// dealt at once (`.damage_amount()`) and the creature the subject; damage of zero is not dealt.
fn a_creature_is_dealt_damage(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Object(hurt), amount, .. } if *amount > 0 && q.chars(*hurt).is(CardType::Creature) => {
            Some(Fired { subject: Target::Object(*hurt), data: [*amount, 0] })
        }
        _ => None,
    }
}

/// "Its controller may exile that many cards from the top of their library. They may play those cards until the end of their next turn":
/// the creature's controller as it was dealt the damage (the creature may have died of it, CR 603.10a), not the Enchantment's, is
/// asked, exiles the cards, and may play them until the end of their own next turn (CR 514.2). A library with fewer cards exiles what it has.
fn exile_that_many_and_play_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(hurt) = item.subject.object() else { return Ok(()) };
    let player = g.state().obj(hurt).controller;
    if effects::ask_may(g, player, item.object, Purpose::Ask(&ASK))? {
        library::exile_top_play_until_end_of_next_turn(g, player, item.matched_damage_amount() as usize);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, a_creature_is_dealt_damage),
        exile_that_many_and_play_them,
        "Whenever a creature is dealt damage, its controller may exile that many cards from the top of their library. They may play those cards until the end of their next turn.",
    )
    .damage_amount()
    .sums_per_subject()],
    ..oracle::EXPEDITED_INHERITANCE
};
