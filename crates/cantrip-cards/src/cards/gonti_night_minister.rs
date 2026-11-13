use cantrip_api::*;
use cantrip_mechanics::{library, tokens};
use cantrip_oracle as oracle;

/// "Whenever a player casts a spell they don't own" (CR 601.2i, 603.2, 108.3): a spell cast (not copied, CR 707.10), by any player,
/// the ability's controller included, whose owner is another player; the ability is about the player who cast it (`StackItem::subject`,
/// "that player").
fn a_player_casts_a_spell_they_dont_own(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if q.obj(*spell).owner != *player => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

/// "That player creates a Treasure token" (CR 111.10a): under the player who cast the spell, not the ability's controller; a
/// player who has left the game since creates nothing (CR 800.4a).
fn that_player_creates_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    if !g.state().in_game(player) {
        return Ok(());
    }
    tokens::create_treasure(g, player)
}

/// "Whenever a creature deals combat damage to one of your opponents" (CR 510.2, 603.2): one trigger for each creature that deals
/// combat damage above zero to a player who is an opponent of the ability's controller. The creature is the subject ("its
/// controller") and the damaged player rides along in `data[0]` ("that opponent").
fn creature_deals_combat_damage_to_one_of_your_opponents(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(p), amount, kind, .. }
            if kind.is_combat() && *amount > 0 && q.chars(*from).is(CardType::Creature) && q.state().opponents(me.controller).any(|o| o == *p) =>
        {
            Some(Fired { subject: Target::Object(*from), data: [u32::from(p.0), 0] })
        }
        _ => None,
    }
}

/// "Its controller looks at the top card of that opponent's library and exiles it face down. They may play that card for as long as
/// it remains exiled. Mana of any type can be spent to cast a spell this way" (CR 406.3, 601.3, 305.1, 609.4b, 118.14): the creature's
/// controller as the ability resolves, or as it last existed if it has left the battlefield (CR 608.2h); the card stays in its
/// owner's exile and the permission lasts until it leaves it, a land included, whatever the type of mana spent. An empty library
/// exiles nothing.
fn exile_the_top_card_to_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    let controller = g.controller(creature);
    let opponent = PlayerId(item.data[0] as u8);
    let grant = PlayGrant { lands: true, any_type: true, ..PlayGrant::CAST };
    library::exile_top_face_down_may_look_and_play(g, opponent, 1, controller, grant);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::SPELL_CAST, a_player_casts_a_spell_they_dont_own),
            that_player_creates_a_treasure,
            "Whenever a player casts a spell they don't own, that player creates a Treasure token.",
        ),
        triggered(
            when_asked(EventMask::DAMAGE, creature_deals_combat_damage_to_one_of_your_opponents),
            exile_the_top_card_to_play,
            "Whenever a creature deals combat damage to one of your opponents, its controller looks at the top card of that opponent's library and exiles it face down. They may play that card for as long as it remains exiled. Mana of any type can be spent to cast a spell this way.",
        ),
    ],
    ..oracle::GONTI_NIGHT_MINISTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
