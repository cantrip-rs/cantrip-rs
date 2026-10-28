use cantrip_api::*;
use cantrip_mechanics::tokens::{self, investigate};
use cantrip_mechanics::{choose_color, exile_and_return_transformed, pump, targets};
use cantrip_oracle as oracle;

fn investigate_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    investigate(g, item.controller)
}

/// "When you draw your third card in a turn": the history is as it was just before this draw, so two already drawn.
fn third_draw(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Drew { player, .. } if *player == q.controller(source) && q.history().drawn(*player) == 2 => Fired::PLAIN,
        _ => None,
    }
}

/// "Exile Tamiyo, then return her to the battlefield transformed under her owner's control."
fn transform_her(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

// --- Tamiyo, Seasoned Scholar -------------------------------------------------------------------------------------

/// "Whenever a creature attacks you or a planeswalker you control" (CR 508.1): each creature, separately. "You" is the
/// delayed ability's controller, who activated the +2 (CR 603.7d), whoever controls Tamiyo now or wherever she is.
fn creature_attacks_you_or_your_planeswalker(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender, .. } => {
            let you = me.controller;
            let at_you = match defender {
                Target::Player(p) => *p == you,
                Target::Object(id) => q.controller(*id) == you && q.chars(*id).is(CardType::Planeswalker),
                _ => false,
            };
            if at_you { Fired::about(Target::Object(*attacker)) } else { None }
        }
        _ => None,
    }
}

/// "It gets -1/-0 until end of turn."
fn attacker_weakens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        pump(g, item.object, creature, -1, 0);
    }
    Ok(())
}

/// +2: until your next turn, the delayed ability watches every attack on you or your planeswalkers (CR 603.7b).
fn plus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.card_id(&BACK);
    g.delayed_trigger(DelayedWhen::UntilNextTurnOf(item.controller), item.object, item.controller, AbilityRef::delayed(me, 0), Target::None);
    Ok(())
}

/// -3: return the card to your hand; if it is a green card, add one mana of any color. "Green" is a word of Tamiyo's text,
/// read through her, so a text-changing effect changes it (CR 612.2).
fn minus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let green = g.chars(card).colors.has(g.words(item.object).color(Color::Green));
    g.return_to_hand(&[card]);
    if green {
        let color = choose_color(g, item.controller, item.object)?;
        g.add_mana(item.controller, color.mana(), 1);
    }
    Ok(())
}

/// -7: draw cards equal to half the number of cards in your library, rounded up; you get an emblem.
fn minus_seven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = g.state().zone(Zone::library(item.controller)).len() as u32;
    g.draw(item.controller, cards.div_ceil(2));
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

pub static EMBLEM: CardDef = CardDef {
    name: "Tamiyo, Seasoned Scholar emblem",
    has_cost: false,
    play_rules: &[PlayRule::MaximumHandSize(None)],
    text: "You have no maximum hand size.",
    ..CardDef::BLANK
};

pub static BACK: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), plus_two, "+2: Until your next turn, whenever a creature attacks you or a planeswalker you control, it gets -1/-0 until end of turn."),
        ability(loyalty(-3), minus_three, "\u{2212}3: Return target instant or sorcery card from your graveyard to your hand. If it's a green card, add one mana of any color.")
            .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD]),
        ability(loyalty(-7), minus_seven, "\u{2212}7: Draw cards equal to half the number of cards in your library, rounded up. You get an emblem with \"You have no maximum hand size.\""),
    ],
    delayed: &[triggered(when_asked(EventMask::ATTACKS, creature_attacks_you_or_your_planeswalker), attacker_weakens, "Whenever a creature attacks you or a planeswalker you control, it gets -1/-0 until end of turn.")],
    ..oracle::TAMIYO_SEASONED_SCHOLAR
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::THIS_ATTACKS, investigate_effect, "Whenever Tamiyo attacks, investigate."),
        triggered(when(EventMask::DREW, third_draw), transform_her, "When you draw your third card in a turn, exile Tamiyo, then return her to the battlefield transformed under her owner's control."),
    ],
    back: Some(&BACK),
    ..oracle::TAMIYO_INQUISITIVE_STUDENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE, &EMBLEM];
