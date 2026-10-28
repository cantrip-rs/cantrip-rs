//! "If {C}{C} was spent to cast it": the intervening "if" of the Elemental Incarnations of Modern Horizons 3 (Vibrance,
//! Wistfulness, Deceit). The mana is counted over the whole payment of the spell (CR 601.2h, 106.1b): mana of that color
//! spent on generic costs and on an evoke cost counts too, and a third mana of the color changes nothing. A permanent that
//! was not cast (put onto the battlefield by an effect, or a copy of a spell, CR 707.10) had nothing spent, and a permanent
//! that copies one has what it was cast with, not what it copies (CR 707.2).

use cantrip_api::*;

/// Whether at least two mana of the kind `M` (a `Mana` as `u8`: `spent::two::<{ Mana::U as u8 }>`) were spent to cast the
/// source of the ability. Read from the object the ability came from, so it still answers when that has left the
/// battlefield (CR 113.7a).
pub fn two<const M: u8>(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).spent.get(Mana::ALL[M as usize]) >= 2
}

/// Opus (an ability word, CR 207.2c): "if five or more mana was spent to cast that spell", where the spell is the one the
/// ability triggered on (`item.subject`). Its mana spent (CR 601.2h) is read from the spell's own object, so it still answers once
/// the spell has been countered or has left the stack (CR 608.2h); a spell cast without paying its mana cost has had none spent.
/// Colorstorm Stallion and Molten-Core Maestro print it.
pub fn five_or_more_mana_spent_on_the_spell(q: &dyn Query, item: &StackItem) -> bool {
    matches!(item.subject, Target::Object(spell) if q.obj(spell).spent.total() >= 5)
}

/// "If mana from a Treasure was spent to cast [the spell]" (CR 601.2h, 106.3): some of the mana paid for the spell, additional
/// costs too, was produced by a Treasure - an artifact with the Treasure subtype as it last was on the battlefield (a Treasure
/// sacrificed for its mana is gone by then, CR 608.2h). A copy was not cast and had nothing spent (CR 707.10), and a spell cast
/// without spending mana has none. Read from the history of the payment, so it still answers once the spell has left the stack.
/// Smaug, Wicked Worm and Mastermind Plum print it as a trigger condition.
pub fn mana_from_a_treasure_spent_to_cast(q: &dyn Query, spell: ObjectId) -> bool {
    q.history().mana_spent_to_cast(spell).any(|(source, _)| source != ObjectId::NONE && q.chars(source).has_subtype(Subtype::Treasure))
}

/// "Whenever you cast a spell, if mana from a Treasure was spent to cast it" (CR 603.2, 603.4): a spell its controller cast, paid
/// for in part with mana from a Treasure. The payment is over before the trigger, and nothing changes it afterwards, so the
/// condition asked as it triggers is the one asked as it resolves. Smaug, Wicked Worm and Mastermind Plum print it.
pub fn you_cast_a_spell_with_mana_from_a_treasure(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && mana_from_a_treasure_spent_to_cast(q, *spell) => Fired::PLAIN,
        _ => None,
    }
}

/// Mana a Treasure produced is told apart from other mana (CR 106.3): its payer chooses whether to spend it, where a permanent asks
/// of the mana spent whether it came from a Treasure (a `CardDef::mana_told_apart` entry). Smaug, Wicked Worm and Mastermind Plum
/// print the question.
pub fn treasure_mana_is_told_apart(q: &dyn Query, _: &Asker, source: ObjectId) -> bool {
    q.chars(source).has_subtype(Subtype::Treasure)
}
