use cantrip_api::*;
use cantrip_mechanics::{attach, choose, effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "A creature that this card could enchant", one, chosen as the ability resolves (CR 608.2d): not targeted, so shroud and
/// hexproof don't matter (the ruling), whoever controls it.
const A_CREATURE: Selection = Selection::of(select::exactly(1), &[select::CREATURE]).called("creature");

/// "As a non-Aura enchantment" (CR 614.12): it enters an enchantment without its enchantment subtype, so it chooses nothing
/// to enchant as it enters (CR 303.4f) and is never an unattached Aura for the state-based actions to put away (CR 704.5m).
/// Aura is the one enchantment subtype the card prints, and an effect that would give it another before it enters exists
/// on no card.
static NON_AURA: EntersAs = EntersAs { loses_subtypes_of: &[CardType::Enchantment], ..EntersAs::NONE };

/// "When enchanted creature dies, that creature's controller chooses a creature that this card could enchant. If the player
/// does, return this card to the battlefield under your control attached to that creature. If they don't, return this card
/// to the battlefield under your control as a non-Aura enchantment. It loses "enchant creature" and gains "At the beginning
/// of that player's upkeep, this enchantment deals 1 damage to that player.""
///
/// The card is the one this Aura became in its owner's graveyard as the creature died or by the state-based action after
/// (CR 400.7f); once it has left that graveyard nothing is returned (the ruling). "That creature's controller" is its
/// controller as it last existed on the battlefield (CR 608.2h); they choose among every creature this card could enchant
/// under the ability's controller (CR 303.4f: protection from black keeps one out, shroud does not), and must choose one if
/// they can (the ruling). It returns under the control of the ability's controller, who controlled it as it left (the
/// ruling). Losing "enchant creature" and gaining the ability follow as one instruction once it is there (CR 613.1f).
fn return_this_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let Some(card) = g.found_aura_after_enchanted_left(item.object, creature) else { return Ok(()) };
    let chooser = g.controller(creature);
    let battlefield: Vec<ObjectId> = g.state().battlefield().to_vec();
    let creatures: Vec<ObjectId> = battlefield
        .into_iter()
        .filter(|&host| g.state().in_game(chooser) && g.chars(host).is(CardType::Creature) && g.could_enter_attached(card, item.controller, host))
        .collect();
    let chosen = choose::select(g, chooser, card, Purpose::Choose, creatures, A_CREATURE)?;
    if let Some(&host) = chosen.first() {
        g.put_onto_battlefield_attached(card, item.controller, host)?;
        return Ok(());
    }
    let Some(permanent) = g.put_onto_battlefield_as(card, item.controller, item, &NON_AURA)? else { return Ok(()) };
    g.end_sentence();
    instruction(g, |g| {
        g.add_effect(permanent, permanent, EffectKind::LoseEnchant(item.ability.card), Duration::Indefinite);
        g.add_effect(permanent, permanent, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
    });
    Ok(())
}

/// "That player" of the gained ability: the controller, as it last existed (CR 608.2h), of the creature whose death returned
/// this permanent, read from the history - the card this permanent entered from, and the move that put that card into the
/// graveyard as an Aura whose enchanted creature had left (CR 400.7f). Only that return gives a permanent the ability.
fn that_player(q: &dyn Query, permanent: ObjectId) -> Option<PlayerId> {
    let log = q.events();
    let card = log.iter().rev().find_map(|r| match r.event {
        Event::ZoneChange { old, new, .. } if new == permanent => Some(old),
        _ => None,
    })?;
    log.iter().rev().find_map(|r| match r.event {
        Event::ZoneChange { new, enchanted_left: Some(host), .. } if new == card => Some(q.controller(host)),
        _ => None,
    })
}

/// "At the beginning of that player's upkeep" (CR 503.1a, 603.2): the trigger is about that player, whoever controls this.
fn that_players_upkeep(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let active = q.state().turn.active;
    match event {
        Event::StepBegan { step: Step::Upkeep } if that_player(q, me.source) == Some(active) => Fired::about(Target::Player(active)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(
            triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
            |g, item| attach::put_counter_on_that_creature(g, item, CounterKind::Plus0Minus1),
            "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.",
        ),
        triggered(
            triggers::ATTACHED_CREATURE_DIES_ABOUT_IT,
            return_this_card,
            "When enchanted creature dies, that creature's controller chooses a creature that this card could enchant. If the player does, return this card to the battlefield under your control attached to that creature. If they don't, return this card to the battlefield under your control as a non-Aura enchantment. It loses \"enchant creature\" and gains \"At the beginning of that player's upkeep, this enchantment deals 1 damage to that player.\"",
        ),
    ],
    delayed: &[triggered(
        when_asked(EventMask::STEP_BEGAN, that_players_upkeep),
        |g, item| effects::damage_that_player(g, item, 1),
        "At the beginning of that player's upkeep, this enchantment deals 1 damage to that player.",
    )],
    ..oracle::TAKKLEMAGGOT
};
