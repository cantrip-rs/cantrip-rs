//! Trigger conditions and abilities many cards print.

use cantrip_api::*;

/// "Whenever this creature enlists a creature" (CR 702.154c): enlist's, kept with the keyword.
pub use crate::enlist::{THIS_ENLISTS, this_enlists};

/// What ward's discard cost asks the player whose spell or ability targeted, the question on that spell or ability.
const WARD_DISCARD: Question =
    Question::new("DiscardUnlessCountered", "Discard a card, or {source} is countered?", "Ward: what it targeted can't be targeted for free", "Discard a card", "Let it be countered").no_default();

/// "When this enters" (CR 603.6a): the entering object's own zone change onto the battlefield.
pub fn this_enters(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Battlefield) => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever another creature or planeswalker you control enters" (CR 603.6a): a different permanent that entered
/// the battlefield under this source's controller's control, and is a creature or planeswalker as it entered. Liliana
/// the Faultless and Liliana the Repentant print it.
pub fn another_creature_or_planeswalker_you_control_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != source
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == q.controller(source)
                && {
                    let chars = q.chars(*new);
                    chars.is(CardType::Creature) || chars.is(CardType::Planeswalker)
                } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "When this enters, if you cast it" (CR 603.4, 603.6a): the permanent entered as the spell that resolved, and that spell
/// was cast - the history says so. A permanent put onto the battlefield, or a copy of a permanent spell (a token, CR 707.10f),
/// was not cast. Lutri, The One Ring and Uldaros Theorix print it.
pub fn this_enters_if_cast(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, new, to, reason: ZoneReason::Resolve, .. }
            if *new == source && to.is(ZoneKind::Battlefield) && q.history().cast_of(*old).is_some() =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever this creature attacks for the first time each turn" (CR 508.1, 603.2): it was declared as an attacker and no
/// earlier declaration this turn had it attack - the history is as it was just before this event, so it holds the earlier
/// ones only. A trigger condition, not a limit on how often the ability triggers (CR 603.2d: Isshin doubles it). Aurelia, the
/// Warleader and Fear of Missing Out print it.
pub fn this_attacks_for_the_first_time_each_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == me.source && !q.history().attacked_this_turn(me.source) => Fired::PLAIN,
        _ => None,
    }
}

/// `this_attacks_for_the_first_time_each_turn` as a trigger condition.
pub const THIS_ATTACKS_FOR_THE_FIRST_TIME_EACH_TURN: Trigger = when_asked(EventMask::ATTACKS, this_attacks_for_the_first_time_each_turn);

/// Valiant: "Whenever this creature becomes the target of a spell or ability you control for the first time each turn"
/// (CR 115.10, 603.2): the creature became the target of something its controller controls, and no earlier event this turn
/// made it the target of something they control - the history is as it was just before this event, so it holds the earlier
/// ones only. A copy of a spell, and a spell or ability whose target was changed to it, make it a target too. Emberheart
/// Challenger and Heartfire Hero print it.
pub fn valiant(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::BecameTarget { target: Target::Object(t), controller, .. } = event else { return None };
    if *t != me.source || *controller != me.controller {
        return None;
    }
    let before = q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::BecameTarget { target: Target::Object(o), controller: c, .. } if o == me.source && c == me.controller)
    });
    if before { None } else { Fired::PLAIN }
}

/// Valiant as a trigger condition: `triggered(VALIANT, ..)`.
pub const VALIANT: Trigger = when_asked(EventMask::BECAME_TARGET, valiant);

/// An intervening "if this [permanent] is tapped" (CR 603.4), checked as the ability triggers and again as it resolves; a
/// permanent that has left the battlefield is judged by the status it last had there (CR 608.2h). Mana Vault and Kona, Rescue
/// Beastie print it.
pub fn this_is_tapped(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).tapped
}

/// An intervening "if it's on the battlefield" (CR 603.4), checked as the ability triggers and again as it resolves: the
/// permanent is still the object that triggered (CR 400.7) and has not left. Animate Dead and Necromancy print it.
pub fn this_is_on_the_battlefield(q: &dyn Query, asker: &Asker) -> bool {
    q.is_on_battlefield(asker.source) && q.obj(asker.source).became.is_none()
}

/// "Whenever a token you control enters" / "whenever one or more tokens you control enter" (CR 603.6a): a token under the
/// source's controller's control as it entered, however it was made (created, or a copy of a permanent spell). The event's
/// subject is the token, so a batch that entered together is the subjects of the matching events (CR 603.2c). Belladonna
/// Took and Caretaker's Talent print it.
pub fn token_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.obj(*new).token && q.controller(*new) == me.controller =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Whenever [a permanent] you control leaves the battlefield during your turn" (CR 603.6c, 603.10a), looked back on: the
/// permanent that left, under the source's controller's control as it last was on the battlefield, to whichever zone it went,
/// while it is that controller's turn. The caller narrows it (another permanent, a type). Suki, Courageous Rescuer and
/// Oni-Cult Anvil print it.
pub fn permanent_you_control_left_on_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<ObjectId> {
    match event {
        Event::ZoneChange { old, from, .. }
            if from.is(ZoneKind::Battlefield) && q.obj(*old).controller == me.controller && q.state().turn.active == me.controller =>
        {
            Some(*old)
        }
        _ => None,
    }
}

/// Landfall (CR 207.2c): a land entering the battlefield under your control, by any means. Icetill Explorer and
/// Tireless Tracker print it.
pub fn landfall(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && q.chars(*new).is(CardType::Land) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "When you unlock this door" on the Room's right half (CR 709.5h): the Room permanent was given that half's unlocked
/// designation by its controller, whether as it entered because the half was cast (CR 709.5d) or after (CR 709.5e-f).
/// Ritual Chamber, Awakening Hall and Cadaver Lab print it.
pub fn you_unlock_this_right_door(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    you_unlock_this_door(q, source, event, DOOR_RIGHT)
}

/// "When you unlock this door" on the Room's left half (CR 709.5h): the same, for the left door. Roaring Furnace, Bottomless
/// Pool and Surgical Suite print it.
pub fn you_unlock_this_left_door(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    you_unlock_this_door(q, source, event, DOOR_LEFT)
}

fn you_unlock_this_door(q: &dyn Query, source: ObjectId, event: &Event, door: u8) -> Option<Fired> {
    match event {
        Event::DoorUnlocked { permanent, door: unlocked, player } if *permanent == source && *unlocked == door && *player == q.controller(source) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "When you cast this spell" (CR 603.2, 601.2i): the spell itself, from wherever it was cast. Sage of the Skies and
/// every cascade card print it.
pub fn cast_this_spell(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if *spell == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you draw a card" (CR 120.3, 603.2): the source's controller drew. Sheoldred, the Apocalypse, Teferi, Hero of
/// Dominaria's emblem and Shang-Chi and the Ten Rings print it.
pub fn you_draw_a_card(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Drew { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever an opponent draws a card" (CR 120.3, 603.2): once for each card an opponent of the source's controller draws
/// (a draw a replacement effect replaced was not made, CR 614.6), that player the trigger's subject. Sheoldred, the
/// Apocalypse and Consecrated Sphinx print it.
pub fn an_opponent_draws_a_card(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Drew { player, .. } if q.state().opponents(me.controller).any(|o| o == *player) => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

/// "Whenever you gain life" (CR 119.3, 603.2): one trigger for each time the source's controller gains life, and none
/// when none is gained (a gain of 0, or life that can't be gained, CR 119.7). Ajani Resolute (and its Pridemate token) and
/// Amalia Benavides Aguirre print it, and Professor Dellian Fel's emblem and Sanguine Bond read the amount ("that
/// much"): the trigger's `data[0]` is the life gained.
pub fn you_gain_life(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta > 0 && *player == me.controller => {
            Some(Fired { subject: Target::None, data: [*delta as u32, 0] })
        }
        _ => None,
    }
}

/// "When the Nth [kind] counter is put on this permanent" (CR 122.6, 603.2): counters of kind `KIND` (`CounterKind::Plan
/// as u8`) were put on it and the Nth was among them - it had fewer than N before and has N or more now, as the event
/// records (`total`). So it triggers once however many are put at once (CR 603.2c), not for a counter after the Nth,
/// whatever a replacement made the number (doubling), for a permanent that enters with N or more of them (CR 122.6), and
/// again if counters were removed and the Nth is put once more, as a Saga's chapters count (CR 714.2b). Political
/// Triumph, Death to Our Enemies and the other Plans print it with plan counters; Shang-Chi and the Ten Rings with +1/+1
/// counters.
pub fn nth_counter_put_on_this<const KIND: u8, const N: i16>(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind, amount, total, .. } if *object == me.source && *kind as u8 == KIND && total - amount < N && *total >= N => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Cascade's effect (CR 702.85a): exile cards from the top of the library until a nonland card with a lesser mana
/// value than the spell's; its controller may cast it without paying its mana cost; then every card exiled and not
/// cast goes to the bottom of the library in a random order. The spell's mana value is its card's, X included, as it
/// is now or as it last was (CR 608.2h). Judged by the card rather than the face cast (an Adventure's cheaper face is
/// not considered).
fn cascade_resolves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let def = g.card(g.chars(item.object).card);
    let limit = def.cost.mana_value() + def.cost.x as u32 * g.obj(item.object).x as u32;

    let (exiled, hit) = crate::library::exile_until_nonland(g, player, &|q, card| q.card(q.chars(card).card).cost.mana_value() < limit);
    // "You may cast it without paying its mana cost if the resulting spell's mana value is less than this spell's": the face
    // cast is the spell, so only a face under the limit may be (CR 702.85a, 709.3b, 715.3b).
    if let Some(card) = hit {
        let ways: Vec<CastWay> = g.cast_ways_during_resolution(card).into_iter().filter(|w| u32::from(w.mana_value) < limit).collect();
        if !ways.is_empty() && matches!(g.ask(player, RequestKind::May { source: card, purpose: Purpose::CastOrBottom })?, Answer::Yes)
            && let Some(way) = crate::choose::cast_way(g, player, card, ways)?
        {
            g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), way: Some(way.way), ..CastTerms::default() })?;
        }
    }

    // What was cast has left exile with its identity; the rest go to the bottom, shuffled among themselves.
    let rest = crate::library::still_exiled(g, &exiled);
    g.put_into_library(&rest, LibraryPlace::BottomInRandomOrder);
    Ok(())
}

/// Annihilator N (CR 702.86a): "Whenever this creature attacks, defending player sacrifices N permanents." The defending
/// player is the one attacked, or the controller of the planeswalker or protector of the battle attacked (CR 508.5).
/// Emrakul, the Aeons Torn and Ulamog's Crusher print it.
pub const fn annihilator<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(ANNIHILATOR, annihilate::<N>, text)
}

/// "Whenever this creature attacks" (CR 508.1m, 603.2): its own attack, whatever it attacks. Aragorn, King of Gondor and
/// Caustic Bronco print it. The `listens` mask is `EventMask::ATTACKS`.
pub fn this_attacks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever this creature is dealt damage" (CR 120.2, 603.2c): the damage it is dealt at once, even if it is lethal, as
/// the trigger's `data[0]` ("that much"); damage of zero is not dealt. The ability is `.sums_per_subject()`, which adds up
/// the events of one combat that hurt it from several sources. Screaming Nemesis and Hexhaven Invigorator print it. The
/// `listens` mask is `EventMask::DAMAGE`.
pub fn this_dealt_damage(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Object(hurt), amount, .. } if *hurt == source && *amount > 0 => {
            Some(Fired { subject: Target::None, data: [*amount as u32, 0] })
        }
        _ => None,
    }
}

/// "Whenever you sacrifice [a permanent]" (CR 701.21a): the permanent `player` sacrificed, as it last was on the
/// battlefield, for the card to judge the quality it names. The ability is `.on_sacrifice()`, and looks back as any ability
/// that triggers on a sacrifice does (CR 603.10a). Tireless Tracker (a Clue) and Biotech Specialist (an artifact) print it.
pub fn sacrificed_by(q: &dyn Query, player: PlayerId, event: &Event) -> Option<ObjectId> {
    match event {
        Event::ZoneChange { old, from, reason: ZoneReason::Sacrifice, .. } if from.is(ZoneKind::Battlefield) && q.controller(*old) == player => Some(*old),
        _ => None,
    }
}

/// "Whenever this creature attacks or blocks" (CR 508.1m, 509.1, 603.2): its own attack or its own block of any
/// attacker. Smuggler's Copter and Bebop & Rocksteady print it.
pub fn this_attacks_or_blocks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == source => Fired::PLAIN,
        // Once, however many attackers it blocks (CR 509.3a): the attacker it was declared to block.
        Event::Blocks { blocker, declared: true, .. } if *blocker == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever this permanent enters or attacks" (CR 603.2, 603.6a): one triggered ability with two conditions, either its own entering the
/// battlefield or its own attack. Overlord of the Balemurk, Overlord of the Mistmoors, Sentinel of the Nameless City, Archon of Cruelty
/// Dragonhawk, Fate's Tempest and Dread Osseosaur print it. The `listens` mask is `ZONE_CHANGE` and `ATTACKS`.
pub fn this_enters_or_attacks(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    this_enters(q, source, event).or_else(|| this_attacks(q, source, event))
}

/// "Whenever this creature attacks" where the ability is about whom it attacks: the player attacked, or the
/// planeswalker or battle (CR 508.5). The defender is the trigger's subject. Generous Plunderer and Otharri, Sun's
/// Glory print it.
pub fn this_attacks_defender(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender, .. } if *attacker == source => Fired::about(*defender),
        _ => None,
    }
}

/// "Whenever equipped creature attacks" (CR 508.1m, 301.5): the creature this Equipment is attached to attacks. Adaptive
/// Omnitool and Argentum Armor print it. `equipped_creature_attacks_subject` carries the creature as the subject.
pub fn equipped_creature_attacks(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.attached_to(source) == Some(*attacker) => Fired::PLAIN,
        _ => None,
    }
}

/// `equipped_creature_attacks` with the attacking creature as the trigger's subject, for an ability that does something
/// to it ("it gains ..."). Bone Sabres and Strength-Testing Hammer print it.
pub fn equipped_creature_attacks_subject(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.attached_to(source) == Some(*attacker) => Fired::about(Target::Object(*attacker)),
        _ => None,
    }
}

/// "Whenever equipped creature deals combat damage to a player" (CR 510.2, 603.2c): the creature this Equipment is attached to
/// dealing more than 0 combat damage to a player; each such event is one player and one trigger. The player is the trigger's
/// subject and the amount of damage its `data[0]` ("that damage"). Bloodforged Battle-Axe, Orcrist, Goblin Cleaver and Buster
/// Sword print it.
pub fn equipped_creature_deals_combat_damage_to_player(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: dealer, target: Target::Player(p), amount, kind, .. }
            if kind.is_combat() && *amount > 0 && q.attached_to(source) == Some(*dealer) =>
        {
            Some(Fired { subject: Target::Player(*p), data: [*amount as u32, 0] })
        }
        _ => None,
    }
}

/// "Whenever this Equipment becomes attached to a creature" (CR 701.3a, 301.5, 603.2): this Equipment (or Aura) became
/// attached to a creature on the battlefield - by equip or reconfigure, by an effect that attaches it, moved from another
/// creature, or entering attached - and not when an attach to the creature it is already on does nothing (CR 701.3b), nor
/// on phasing in (CR 702.26j). The creature is the trigger's subject ("that creature"). Assimilation Aegis, Blade of
/// Shared Souls, Enormous Energy Blade, Inchblade Companion and Killer Cosplay print it.
pub fn this_becomes_attached_to_a_creature(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attached { attachment, to: Target::Object(host) }
            if *attachment == source && q.is_on_battlefield(*host) && q.chars(*host).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*host))
        }
        _ => None,
    }
}

/// `this_becomes_attached_to_a_creature` as a trigger condition: `triggered(THIS_BECOMES_ATTACHED_TO_A_CREATURE, ..)`.
pub const THIS_BECOMES_ATTACHED_TO_A_CREATURE: Trigger = when(EventMask::ATTACHED, this_becomes_attached_to_a_creature);

/// "Whenever enchanted player is attacked" (CR 508.1m, 303.4): a creature attacks the player this Curse is attached to,
/// directly. The attacking creature is the trigger's subject. Curse of Opulence and Curse of Predation print it.
pub fn enchanted_player_attacked(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(p), .. } if q.obj(source).attached_player == Some(*p) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "Whenever you attack" (CR 508.1m): the controller declares attackers, whatever their number. Guide of Souls and Inti,
/// Seneschal of the Sun print it.
pub fn you_attack(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::AttackersDeclared { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

fn annihilate<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    annihilate_n(g, item, N)
}

/// Annihilator's resolution (CR 702.86a) for an N worked out as it resolves: the defending player sacrifices `n`
/// permanents. Ulamog, the Defiler's "annihilator X, where X is the number of +1/+1 counters on it" calls it, with its
/// own `triggered(ANNIHILATOR, ..)`.
pub fn annihilate_n(g: &mut dyn Game, item: &StackItem, n: u8) -> Result<(), Suspend> {
    // The defending player of the creature with annihilator (CR 508.5), whatever has become of what it attacked.
    let Some(defending) = g.defending_player(item.object) else { return Ok(()) };
    crate::sacrifice_chosen(g, defending, item.object, n, |_| true).map(drop)
}

/// Annihilator's trigger (CR 702.86a): "whenever this creature attacks", about the player or permanent it attacks.
pub const ANNIHILATOR: Trigger = when(EventMask::ATTACKS, this_attacks_defender);

/// Cascade (CR 702.85a): "When you cast this spell, exile cards from the top of your library until you exile a nonland
/// card that costs less. You may cast it without paying its mana cost. Put all cards exiled this way that weren't cast
/// on the bottom of your library in a random order." Bloodbraid Challenger and Bloodbraid Elf print it.
pub const CASCADE: AbilityDef = triggered(CAST_THIS_SPELL, cascade_resolves, "Cascade");

/// "When you cycle this card" (CR 702.29c): "When you discard this card to pay an activation cost of a cycling ability",
/// typecycling's included (CR 702.29f), from whatever zone the card winds up in (`WHERE_A_DISCARD_ENDS`). `Event::Cycled`
/// names the card where the discard put it, which is the ability's source. The ability is about the card cycled; `data[0]`
/// is the X announced for the cycling ability that discarded it (CR 107.3a), read off that ability where it stands on the
/// stack as the trigger is collected (CR 602.2b, 603.2) - its object is the card as it was in the hand (CR 400.7): the X
/// of a cycling cost such as Shark Typhoon's {X}{1}{U}, 0 for any other. Shark Typhoon, Krosan Tusker, Renewed Faith and
/// Decree of Pain print it.
pub fn you_cycle_this_card(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Cycled { card, player } = *event else { return None };
    if player != me.controller || card != me.source {
        return None;
    }
    let x = q.state().stack.iter().rev().find(|i| i.kind == StackKind::Activated && q.obj(i.object).became == Some(card)).map_or(0, |i| i.x);
    Some(Fired { subject: Target::Object(card), data: [x as u32, 0] })
}

pub const YOU_CYCLE_THIS_CARD: Trigger = when_asked(EventMask::CYCLED, you_cycle_this_card).functions_in(WHERE_A_DISCARD_ENDS);

fn cycle_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// Cycling [cost] (CR 702.29a): "[cost], Discard this card: Draw a card", from the hand. `text` is the ability's
/// full text, which a scenario's `Ability:` matches.
pub const fn cycling(mana: ManaCost, text: &'static str) -> AbilityDef {
    ability(Cost::mana(mana).and(DISCARD_THIS), cycle_draw, text)
        .from(ZoneKind::Hand)
        .cycling()
}

/// Whether the creature declared as an attacker is the only one (CR 506.5: "a creature attacks alone if it's the only creature
/// declared as an attacker during the declare attackers step").
fn attacks_alone(q: &dyn Query) -> bool {
    q.state().attackers.len() == 1
}

/// "Whenever a creature you control attacks alone" (CR 506.5, 508.1m): the only creature declared as an attacker, controlled
/// by this ability's controller. The ability is about that attacker. Exalted, Agent 13, Sharon Carter and Agents of
/// S.H.I.E.L.D. print it.
pub fn creature_you_control_attacks_alone(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if attacks_alone(q) && q.controller(*attacker) == me.controller => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "Whenever this creature attacks alone" (CR 506.5, 508.1m): its own attack, the only creature declared as an attacker. Luke
/// Cage, Power Man and Bob, Reluctant HYDRA Agent print it.
pub fn this_attacks_alone(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == source && attacks_alone(q) => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever equipped creature attacks alone" (CR 506.5, 508.1m, 301.5): the creature this Equipment is attached to is the only
/// creature declared as an attacker. The ability is about that creature ("untap it"). S.H.I.E.L.D. Spy Kit and Bilbo's Ring
/// print it.
pub fn equipped_creature_attacks_alone(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if attacks_alone(q) && q.attached_to(source) == Some(*attacker) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "Whenever a creature you control attacks a player alone" (CR 506.5, 508.1): the only creature declared as an attacker,
/// attacking a player rather than a planeswalker or battle, and controlled by this ability's controller. The ability is
/// about that attacker. Jiang Yanggu, Alone and Yuriko, Blade of the Mighty print it.
pub fn creature_you_control_attacks_a_player_alone(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(_), .. }
            if q.state().attackers.len() == 1 && q.controller(*attacker) == q.controller(source) =>
        {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "That creature gets +1/+1 until end of turn" (CR 611.2a): the creature the trigger is about, as it resolves. Exalted and
/// Agents of S.H.I.E.L.D. print it.
pub fn subject_gets_plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        crate::pump(g, item.object, creature, 1, 1);
    }
    Ok(())
}

/// Exalted (CR 702.83a): "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn."
/// Ignoble Hierarch and Noble Hierarch print it word for word.
pub const EXALTED: AbilityDef = triggered(CREATURE_YOU_CONTROL_ATTACKS_ALONE, subject_gets_plus_one, "Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.");

/// "When this creature enters, if its squad cost was paid": an unpaid cast, or a token copy, does not trigger at all.
fn entered_squadded(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Battlefield) && q.obj(source).kicked > 0 => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// Create a token that's a copy of it for each time its squad cost was paid, all at once, from its copiable values
/// as the ability resolves.
fn squad_copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let times = g.obj(item.object).kicked as u32;
    if times > 0 {
        g.create_token_copies(item.object, item.controller, times)?;
    }
    Ok(())
}

/// Squad (CR 702.157a): the enters trigger. Securitron Squadron and Sicarian Infiltrator print it word for word; the
/// additional cost is the card's `kicker`.
pub const SQUAD: AbilityDef = triggered(when(EventMask::ZONE_CHANGE, entered_squadded), squad_copies, "Squad: When this creature enters, if its squad cost was paid, create a token that's a copy of it for each time its squad cost was paid.");

use cantrip_api::intrinsic::ward_matches;

/// Ward {N} (CR 702.21a): "Whenever this becomes the target of a spell or ability an opponent controls, counter it
/// unless that player pays {N}." A creature granted ward (Lavaspur Boots) has it from the granting card's delayed
/// abilities; Kappa Cannoneer prints it with {4}. Defined with the rules' own abilities, as a cloaked permanent has it
/// (CR 701.58a).
pub use cantrip_api::intrinsic::ward;

/// Ward's life cost, paid or not (CR 702.21a, 118.12a): the payer may pay `amount` life when they have that much (CR
/// 119.4), in one payment, and the spell or ability that targeted is countered unless they do.
fn ward_pay_life(g: &mut dyn Game, item: &StackItem, life: u16) -> Result<(), Suspend> {
    let Target::Object(by) = item.subject else { return Ok(()) };
    let payer = PlayerId(item.data[0] as u8);
    let paid = g.state().player(payer).life >= i32::from(life)
        && g.may_pay(payer, item.object, by, Cost { life: u32::from(life), ..Cost::NONE }, &crate::COUNTERED_UNLESS_PAID)?;
    if !paid {
        g.counter_item(by, item.data[1] as usize);
    }
    Ok(())
}

fn ward_life_effect<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    ward_pay_life(g, item, u16::from(N))
}

/// Ward--Pay N life (CR 702.21a): "Whenever this becomes the target of a spell or ability an opponent controls, counter it
/// unless that player pays N life." Sedgemoor Witch prints it with 3.
pub const fn ward_life<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_life_effect::<N>, text)
        .ward()
}

fn ward_life_equal_to_power_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The ward source's power as the ward ability resolves, or as it last was on the battlefield (CR 113.7a, 608.2h).
    // A power below 0 is 0 life (CR 107.1b); any other is a cost of that much life exactly, whatever a power can be.
    let power = g.chars(item.object).power.unwrap_or(0).max(0);
    ward_pay_life(g, item, power.unsigned_abs())
}

/// Ward--Pay life equal to this creature's power (CR 702.21a, 118.12): "Whenever this becomes the target of a spell or
/// ability an opponent controls, counter it unless that player pays life equal to its power." The power is worked out as the
/// ward ability resolves, from the creature as it last existed if it has left the battlefield. Raubahn, Bull of Ala Mhigo
/// prints it.
pub const fn ward_life_equal_to_power(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_life_equal_to_power_effect, text)
        .ward()
}

fn ward_discard_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(by) = item.subject else { return Ok(()) };
    let payer = PlayerId(item.data[0] as u8);
    // A player with no card in hand cannot pay and is not asked, nor is one whom the ward's controller's abilities can't make
    // discard (CR 101.2; Tamiyo, Collector of Tales's ruling: "something happens unless you discard" can't be paid); one who
    // can is asked whether to, and then which card.
    if !g.state().zone(Zone::hand(payer)).is_empty()
        && may_cause(&*g, item.controller, payer, CauseKind::Discard)
        && matches!(g.ask(payer, RequestKind::May { source: by, purpose: Purpose::Ask(&WARD_DISCARD) })?, Answer::Yes)
    {
        crate::discard_chosen(g, payer, item.object, 1)?;
        return Ok(());
    }
    g.counter_item(by, item.data[1] as usize);
    Ok(())
}

/// Ward--Discard a card (CR 702.21a, 118.12): "Whenever this becomes the target of a spell or ability an opponent controls,
/// counter it unless that player discards a card." Sunset Saboteur prints it.
pub const fn ward_discard(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_discard_effect, text)
        .ward()
}

/// What ward's sacrifice cost asks the player whose spell or ability targeted, the question on that spell or ability.
fn ward_sacrifice_question(n: u8) -> &'static Question {
    const DETAIL: &str = "Ward: what it targeted can't be targeted for free";
    const TWO: Question = Question::new(
        "SacrificeUnlessCountered",
        "Sacrifice two permanents, or {source} is countered?",
        DETAIL,
        "Sacrifice two permanents",
        "Let it be countered",
    )
    .amount(2)
    .no_default();
    const THREE: Question = Question::new(
        "SacrificeUnlessCountered",
        "Sacrifice three permanents, or {source} is countered?",
        DETAIL,
        "Sacrifice three permanents",
        "Let it be countered",
    )
    .amount(3)
    .no_default();
    const ANY: Question = Question::new(
        "SacrificeUnlessCountered",
        "Sacrifice the permanents, or {source} is countered?",
        DETAIL,
        "Sacrifice them",
        "Let it be countered",
    )
    .no_default();
    match n {
        2 => &TWO,
        3 => &THREE,
        _ => &ANY,
    }
}

/// What ward's "sacrifice N nonland permanents" asks the player whose spell or ability targeted.
fn ward_sacrifice_nonland_question(n: u8) -> &'static Question {
    const DETAIL: &str = "Ward: what it targeted can't be targeted for free";
    const THREE: Question = Question::new(
        "SacrificeNonlandUnlessCountered",
        "Sacrifice three nonland permanents, or {source} is countered?",
        DETAIL,
        "Sacrifice three nonland permanents",
        "Let it be countered",
    )
    .amount(3)
    .no_default();
    const ANY: Question =
        Question::new("SacrificeNonlandUnlessCountered", "Sacrifice the nonland permanents, or {source} is countered?", DETAIL, "Sacrifice them", "Let it be countered")
            .no_default();
    match n {
        3 => &THREE,
        _ => &ANY,
    }
}

/// Ward--Sacrifice `n` of a kind of permanent (CR 702.21a, 118.12): counters the spell or ability that targeted unless the
/// player who controls it sacrifices `n` permanents they control that `kind` admits. A cost is paid only with what it needs
/// (CR 118.3): a player controlling fewer than `n` such permanents that they can sacrifice can't pay and is not asked - one
/// that can't be sacrificed pays nothing, and none does for a player whom the ward's controller's abilities can't make
/// sacrifice (CR 101.2; Tamiyo, Collector of Tales's ruling: "something happens unless you sacrifice" can't be paid). The payer then chooses which `n`, all sacrificed at once.
fn ward_sacrifice_among(
    g: &mut dyn Game,
    item: &StackItem,
    n: u8,
    kind: fn(&dyn Query, &Asker, ObjectId) -> bool,
    question: &'static Question,
) -> Result<(), Suspend> {
    let Target::Object(by) = item.subject else { return Ok(()) };
    let payer = PlayerId(item.data[0] as u8);
    let asker = g.item_asker(item, payer);
    // Only what the payer can sacrifice pays (CR 118.3, `Game::can_sacrifice`): a permanent that can't be sacrificed, or any
    // at all when the ward's controller can't cause them to sacrifice (CR 101.2), is no resource for it.
    let eligible: Vec<ObjectId> = g.permanents_of(payer).into_iter().filter(|&id| kind(&*g, &asker, id) && g.can_sacrifice(payer, id)).collect();
    if eligible.len() >= n as usize
        && g.ask(payer, RequestKind::May { source: by, purpose: Purpose::Ask(question) })? == Answer::Yes
    {
        return crate::sacrifice_one_of(g, payer, item.object, n, eligible).map(drop);
    }
    g.counter_item(by, item.data[1] as usize);
    Ok(())
}

fn any_permanent(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

fn ward_sacrifice_effect<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    ward_sacrifice_among(g, item, N, any_permanent, ward_sacrifice_question(N))
}

fn ward_sacrifice_nonland_effect<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    ward_sacrifice_among(g, item, N, crate::targets::filters::nonland, ward_sacrifice_nonland_question(N))
}

/// Ward--Sacrifice N nonland permanents (CR 702.21a, 118.12): "Whenever this becomes the target of a spell or ability an
/// opponent controls, counter it unless that player sacrifices N nonland permanents." Valgavoth, Terror Eater prints it with
/// three.
pub const fn ward_sacrifice_nonland<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_sacrifice_nonland_effect::<N>, text)
        .ward()
}

/// Ward--Sacrifice N permanents (CR 702.21a, 118.12): "Whenever this becomes the target of a spell or ability an opponent
/// controls, counter it unless that player sacrifices N permanents." Emrakul, the Exigent Doom prints it with three,
/// Ulamog, the Defiler with two.
pub const fn ward_sacrifice<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_sacrifice_effect::<N>, text)
        .ward()
}

/// What ward's "sacrifice a Food" asks the player whose spell or ability targeted, the question on that spell or ability.
const WARD_SACRIFICE_FOOD: Question = Question::new(
    "SacrificeFoodUnlessCountered",
    "Sacrifice a Food, or {source} is countered?",
    "Ward: what it targeted can't be targeted for free",
    "Sacrifice a Food",
    "Let it be countered",
)
.no_default();

fn ward_sacrifice_a_food_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    ward_sacrifice_among(g, item, 1, crate::targets::filters::food, &WARD_SACRIFICE_FOOD)
}

/// Ward--Sacrifice a Food (CR 702.21a, 118.12): "Whenever this becomes the target of a spell or ability an opponent controls,
/// counter it unless that player sacrifices a Food." Ygra, Eater of All prints it.
pub const fn ward_sacrifice_a_food(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::BECAME_TARGET, ward_matches), ward_sacrifice_a_food_effect, text)
        .ward()
}

/// "Whenever you cast a spell" (CR 601.2i, 603.2): Paradox Engine, Showdown of the Skalds' chapter III, the emblems of Chandra, Torch of Defiance, Chandra, Chill
/// of Compliance and Jace, Vryn's Prodigy print it.
pub fn you_cast_a_spell(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you cast a noncreature spell" (CR 603.2): prowess, Displacer Kitten and Dragon's Rage Channeler print it.
pub fn noncreature_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && !q.chars(*spell).is(CardType::Creature) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever you cast a spell that targets a creature you control" (CR 603.2, 601.2c): a question about the spell as it is
/// cast, whose targets are public from the moment it was announced. One trigger for the spell however many of the creatures it
/// targets (CR 603.2c is about events: one cast is one event). A copy is not cast (CR 707.10), so it sets nothing off; a spell
/// that targets only a creature an opponent controls, a player, or a card elsewhere does not. Ms. Marvel, Kamala Khan, Colleen
/// Wing, Street Samurai and Mockingbird, Ace Agent print it.
pub fn you_cast_a_spell_that_targets_a_creature_you_control(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if let Event::SpellCast { spell, player, .. } = event
        && *player == me.controller
        && let Some(item) = q.state().stack.iter().find(|i| i.kind == StackKind::Spell && i.object == *spell)
        && q.state().targets(item).iter().filter_map(|t| t.object()).any(|id| {
            crate::targets::permanent(q, id) && q.chars(id).is(CardType::Creature) && q.controller(id) == *player
        })
    {
        Fired::PLAIN
    } else {
        None
    }
}

pub const YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL: Trigger =
    when_asked(EventMask::SPELL_CAST, you_cast_a_spell_that_targets_a_creature_you_control);

/// "Whenever a player casts a noncreature spell" (CR 603.2, 601.2i): any player's, the ability's controller included, read as
/// the spell stands on the stack. The ability is about the player who cast it (`StackItem::subject`). Magebane Lizard and Mai,
/// Scornful Striker print it.
pub fn player_casts_noncreature_spell(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if !q.chars(*spell).is(CardType::Creature) => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

/// "Whenever an opponent casts a spell with mana value `N` or less" (CR 601.2i, 603.2, 202.3): a spell cast (not a copy, CR 707.10,
/// and not an ability) by a player other than the source's controller, with the mana value it has on the stack, where an {X} in
/// its cost counts as the X chosen for it (CR 202.3e). The ability is about the player who cast it (`StackItem::subject`, "that
/// player"). Solarium Sentry (2) and Scalding Viper (3) print it.
pub fn opponent_casts_spell_with_mana_value_at_most<const N: u32>(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player != q.controller(source) && q.mana_value(*spell) <= N => {
            Fired::about(Target::Player(*player))
        }
        _ => None,
    }
}

pub const A_PLAYER_CASTS_A_NONCREATURE_SPELL: Trigger = when(EventMask::SPELL_CAST, player_casts_noncreature_spell);

/// "Whenever you cast an instant or sorcery spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10), by this
/// ability's controller, read as it stands on the stack. The spell rides along as the trigger's subject ("that spell").
/// Colorstorm Stallion's opus and Stormchaser's Talent's third level print it.
pub fn you_cast_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player == me.controller && (q.chars(*spell).is(CardType::Instant) || q.chars(*spell).is(CardType::Sorcery)) =>
        {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// "Whenever an opponent casts an instant or sorcery spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10), by a
/// player other than the ability's controller, read as it stands on the stack. The spell rides along as the trigger's subject.
/// Arasta of the Endless Web and Wandering Archaic print it.
pub fn opponent_casts_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. }
            if *player != me.controller && (q.chars(*spell).is(CardType::Instant) || q.chars(*spell).is(CardType::Sorcery)) =>
        {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// "Whenever you cast a spell from exile" (CR 601.2a, 603.2): the spell's controller is the ability's controller and the zone
/// it was cast from, recorded on it, is exile. Appa, Steadfast Guardian and Quintorius Kand print it.
pub fn you_cast_a_spell_from_exile(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.obj(*spell).cast_from == Some(ZoneKind::Exile) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn prowess_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::pump(g, item.object, item.object, 1, 1);
    Ok(())
}

/// Prowess (CR 702.108a): "Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn", as a
/// real triggered ability. The Otter token of Stormchaser's Talent and the Monk token of Cori-Steel Cutter carry it.
pub const PROWESS: AbilityDef = triggered(when_asked(EventMask::SPELL_CAST, noncreature_spell_you_cast), prowess_pump, "Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)");

/// The instant or sorcery spell `player` cast or copied, if this event is one (CR 601.2i, 707.10): magecraft's trigger event,
/// and what Veyran, Voice of Duality's "you casting or copying an instant or sorcery spell" names. A copy of a spell is not
/// cast (CR 707.10), and a copy of a card that is no spell on the stack is not "copying a spell", so only a copy on the stack
/// counts. The spell is read as it is on the stack.
pub fn instant_or_sorcery_cast_or_copied(q: &dyn Query, event: &Event) -> Option<(ObjectId, PlayerId)> {
    let (spell, player) = match *event {
        Event::SpellCast { spell, player, .. } => (spell, player),
        Event::Copied { copy, player, .. } if q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == copy) => (copy, player),
        _ => return None,
    };
    let c = q.chars(spell);
    (c.is(CardType::Instant) || c.is(CardType::Sorcery)).then_some((spell, player))
}

/// Magecraft (CR 207.2c, an ability word): "Whenever you cast or copy an instant or sorcery spell", one ability that triggers
/// on either. Witherbloom Apprentice, Sedgemoor Witch and Veyran, Voice of Duality print it.
pub fn you_cast_or_copy_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match instant_or_sorcery_cast_or_copied(q, event) {
        Some((_, player)) if player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "When this is put into a graveyard from the battlefield" (CR 700.4 for a creature: "dies"), looked back on: the
/// ability is the object's as it last existed there (CR 603.10a).
pub fn this_dies(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. } if *old == source && from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "When this leaves the battlefield" (CR 603.6c, 603.10a), looked back on whichever zone it goes to. Animate Dead and
/// Cryogen Relic print it.
pub fn this_leaves(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. } if *old == source && from.is(ZoneKind::Battlefield) => Fired::PLAIN,
        _ => None,
    }
}

/// "At the beginning of your [step]" (CR 503.1a, 603.2): `step` began on its controller's turn.
fn your_step(q: &dyn Query, me: &Asker, event: &Event, step: Step) -> Option<Fired> {
    match event {
        Event::StepBegan { step: began } if *began == step && q.state().turn.active == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you draw your second card each turn" (CR 120.3, 603.2): the draw of a card by the controller when exactly one
/// card was drawn by them earlier this turn (the history holds the draws before the one being matched), in any step. Raven
/// Eagle and Tiger-Seal print it.
pub fn you_draw_your_second_card(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Drew { player, .. } if *player == me.controller && crate::history::drawn_this_turn(q, *player) == 1 => Fired::PLAIN,
        _ => None,
    }
}

/// "At the beginning of your upkeep" (CR 503.1a). Dark Confidant and Court of Garenbrig print it.
pub fn your_upkeep(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::Upkeep)
}

/// "At the beginning of your draw step" (CR 504.1, 603.2). Mana Vault and Bandit's Talent print it.
pub fn your_draw_step(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::Draw)
}

/// "At the beginning of each opponent's upkeep" (CR 503.1a, 603.2): the upkeep began on another player's turn. Abhorrent
/// Oculus, White Plume Adventurer and Lorehold, the Historian print it.
pub fn each_opponents_upkeep(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Upkeep } if q.state().turn.active != me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "At the beginning of your first main phase" or, as older cards say it, "your precombat main phase" (CR 505.1a: only the
/// first main phase of a turn is the precombat one). Coalition Relic and both faces of Ashling, Rekindled print it.
pub fn your_first_main_phase(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::PrecombatMain)
}

/// "At the beginning of your second main phase" (CR 505.1a, 603.2): the postcombat main phase of a turn of the controller's,
/// the only second main phase a turn has (an additional combat phase is followed by that one main phase, CR 500.8). Fireglass
/// Mentor and Kona, Rescue Beastie print it.
pub fn your_second_main_phase(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::PostcombatMain)
}

/// "At the beginning of combat on your turn" (CR 507.1). Luminarch Aspirant and Ouroboroid print it.
pub fn beginning_of_your_combat(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::BeginCombat)
}

/// "At the beginning of your end step" (CR 513.1). Dawn of a New Age and Feywild Caretaker print it.
pub fn your_end_step(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    your_step(q, me, event, Step::End)
}

/// "At the beginning of each end step" (CR 513.1, 603.2): any player's end step began. Gau, Feral Youth and Octomancer print it.
pub fn each_end_step(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::End } => Fired::PLAIN,
        _ => None,
    }
}

/// "At the beginning of each upkeep" (CR 503.1a): any player's upkeep step began. Command the Stage and Ginger, Queen of
/// Sweets print it.
pub fn each_upkeep(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Upkeep } => Fired::PLAIN,
        _ => None,
    }
}

/// "At end of combat" (CR 511.2): it triggers as the end of combat step begins, in any player's turn - each combat phase's,
/// where a turn has more than one. Kytheon, Hero of Akros, Clockwork Avian and Kjeldoran Frostbeast print it.
pub fn end_of_combat(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::EndOfCombat } => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever a creature you control enters" (CR 603.6a): any creature, this one included, under your control as it
/// entered. Aura Shards and Warleader's Call print it.
pub fn creature_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever a creature you control with power 4 or greater enters" (CR 603.6a): any creature, this one included, under your
/// control as it entered, whose power as it entered is 4 or more - the static abilities that apply to it and the counters it
/// entered with count (the 2019-05-03 ruling on Kiora, Behemoth Beckoner). Garruk, Curse Breaker and Kiora, Behemoth
/// Beckoner print it.
pub fn creature_with_power_four_or_more_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    creature_you_control_enters(q, me, event)?;
    let Event::ZoneChange { new, .. } = event else { return None };
    if q.chars(*new).power.is_some_and(|p| p >= 4) { Fired::PLAIN } else { None }
}

/// "Whenever a legendary creature you control enters" (CR 603.6a): any legendary creature, this one included, under your
/// control as it entered. The Irencrag and Tinybones Joins Up print it.
pub fn legendary_creature_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && {
            let c = q.chars(*new);
            c.is(CardType::Creature) && c.supertypes.legendary
        } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever a Dragon you control enters" (CR 603.6a): any permanent that is a Dragon as it enters, under your control; the
/// word is read through the words of the object whose text says it (CR 612). Sarkhan, Dragon Ascendant and Roiling
/// Dragonstorm print it.
pub fn dragon_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Dragon)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever another [subtype] you control enters" (CR 603.6a), `SUBTYPE` a `Subtype` as `u16` (`lands::sub`): a permanent
/// other than this one, under your control as it entered, that is that creature type (a token too), the type read through
/// the words of the source's text (CR 612). Thalia's Lieutenant (Humans) and Youthful Valkyrie (Angels) print it.
pub fn another_of_subtype_you_control_enters<const SUBTYPE: u16>(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && q.chars(*new).has_subtype(crate::lands::word(q, me.source, SUBTYPE)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever a creature you control attacks" (CR 508.3a, 603.2c): declaring attackers names each attacker in its own event, so
/// the ability triggers once for each creature of its controller's that attacks, and is about that attacker. A creature put onto
/// the battlefield attacking was never declared (CR 508.4) and triggers nothing. Shared Animosity, Connecting the Dots and
/// Ingris Stingerquill print it.
pub fn creature_you_control_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller => Fired::about(Target::Object(*attacker)),
        _ => None,
    }
}

/// "Whenever a creature an opponent controls enters" (CR 603.6a): any creature, token or not, under the control of a player
/// other than this ability's controller as it entered. The ability is about that player (`Fired::about`), for "deals 1 damage
/// to that player". Authority of the Consuls and Gideon the Oathless print it.
pub fn opponents_creature_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) != me.controller && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::about(Target::Player(q.controller(*new)))
        }
        _ => None,
    }
}

/// "Whenever an artifact you control enters" (CR 603.6a): any artifact, this one included if it is one, under your control as
/// it entered. Tezzeret, Cruel Captain and Chandra, Spark Hunter's emblem print it; Dowsing Device's "this artifact or another
/// artifact you control enters" is the same event.
pub fn artifact_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && q.chars(*new).is(CardType::Artifact) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever another creature you control enters" (CR 603.6a): a creature other than this permanent, under your control
/// as it entered. The entering creature is the trigger's subject. Aunt May and Guide of Souls print it.
pub fn another_creature_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source
                && to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Whenever a creature dies" (CR 700.4, 603.10a), looked back on: any creature, anyone's, as it last was on the battlefield.
/// The ability carries the player who then controlled it (CR 608.2h). On a permanent or an emblem alike (the Sephiroth,
/// One-Winged Angel emblem), a dies ability is a leaves-the-battlefield ability wherever it functions from, and looks back
/// (`CardTable::looks_back`). Fecundity and Ashes of the Abhorrent print it.
pub fn creature_dies(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.chars(*old).is(CardType::Creature) =>
        {
            Fired::about(Target::Player(q.controller(*old)))
        }
        _ => None,
    }
}

/// "Whenever another creature dies" (CR 700.4, 603.10a): a creature other than the source, using its last-known
/// characteristics. Battle-Scarred Survivalist and Sephiroth, Fabled SOLDIER print it.
pub fn another_creature_dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if *old == source => None,
        _ => creature_dies(q, source, event),
    }
}

/// "Whenever a creature you control dies" (CR 700.4, 603.10a), looked back on: the creature as it last was on the
/// battlefield, under the controller it last had, on a permanent or an emblem alike. Grave Venerations and Warhost's Frenzy
/// print it.
pub fn creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Creature)
                && q.obj(*old).controller == me.controller =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever you cast a creature spell" (CR 603.2): its controller's spells only, judged on the spell as it is on the stack
/// (CR 601.2i), so a card that is a creature only there counts. Bontu's Monument, Glimpse of Nature, Silk, Web Weaver and
/// Spider-Man India print it.
pub fn creature_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.chars(*spell).is(CardType::Creature) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever you cast an artifact spell" (CR 603.2). Pinnacle Emissary and Ravenous Robots print it.
pub fn artifact_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.chars(*spell).is(CardType::Artifact) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// `deals_combat_damage_to_player` for an ability about the damaged player or the damage: the player is the trigger's
/// subject and the amount of damage its `data[0]`. Etali, Primal Conqueror and Fallen Shinobi print it.
pub fn deals_combat_damage_to_player_subject(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(p), amount, kind, .. } if kind.is_combat() && *from == source && *amount > 0 => {
            Some(Fired { subject: Target::Player(*p), data: [*amount as u32, 0] })
        }
        _ => None,
    }
}

/// "Whenever this creature deals combat damage to a player" (CR 510.2, 603.2): its own damage, greater than zero.
/// Bejeweled Warg prints it; Carnage, Crimson Chaos's granted ability has the same condition.
pub fn deals_combat_damage_to_player(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(_), amount, kind, .. } if kind.is_combat() && *from == source && *amount > 0 => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Whenever this creature deals combat damage to a creature" (CR 510.2, 603.2): once for each creature it damages, judged
/// as the damage is dealt, with "that creature" the trigger's subject. Stinkweed Imp prints it, and Kaldra Compleat grants it.
pub fn deals_combat_damage_to_a_creature(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Object(id), amount, kind, .. }
            if kind.is_combat() && *from == source && *amount > 0 && q.chars(*id).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*id))
        }
        _ => None,
    }
}

/// "Whenever a creature you control deals combat damage to a player" (CR 510.2, 603.2): one trigger for each creature and
/// player, with "that player" fixed as the trigger's subject. Vraska, Golgari Queen's emblem and Enduring Curiosity print it.
pub fn creature_you_control_deals_combat_damage_to_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, target: Target::Player(p), amount, kind, .. }
            if kind.is_combat() && *amount > 0 && q.controller(*from) == me.controller && q.chars(*from).is(CardType::Creature) =>
        {
            Fired::about(Target::Player(*p))
        }
        _ => None,
    }
}

/// "Whenever you commit a crime" (CR 700.13): the ability's controller cast a spell, activated an ability or put a
/// triggered ability on the stack that targets an opponent, anything an opponent controls and/or a card in an opponent's
/// graveyard - once for that spell or ability, however many such targets it has. Magda, the Hoardmaster, Raven of Fell
/// Omens, Blood Hustler and Deepmuck Desperado print it.
pub fn you_commit_a_crime(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CrimeCommitted { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you commit a crime" as a trigger condition: `triggered(YOU_COMMIT_A_CRIME, ..)`.
pub const YOU_COMMIT_A_CRIME: Trigger = when_asked(EventMask::CRIME, you_commit_a_crime);

/// "Whenever you discard a card" (CR 701.9a): each card of a discard is its own event, so the ability triggers once for
/// each card discarded, and the card, in the graveyard, is what it is about. With `.once_per_batch()` it is "whenever you
/// discard one or more cards" (CR 603.2c). Ivora, Currency Converter, Marauding Mako, Inti and Cool but Rude print it.
pub fn you_discard(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Discarded { player, card, .. } if *player == me.controller => Fired::about(Target::Object(*card)),
        _ => None,
    }
}

pub const YOU_DISCARD: Trigger = when_asked(EventMask::DISCARDED, you_discard);

/// "Whenever you flip a coin" (CR 705.1): any flip its controller makes, won or lost.
pub fn you_flip_a_coin(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CoinFlipped { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you win a coin flip" (CR 705.2): a flip its controller wins.
pub fn you_win_a_flip(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::CoinFlipped { player, result: CoinFlip::Won } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you roll a die" (CR 706.2): a die its controller rolls, whatever its sides.
pub fn you_roll_a_die(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::DieRolled { player, .. } if *player == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever you scry" (CR 701.22, 603.2): its controller scried, once the scry was complete (CR 701.22d), whatever made
/// them. Scry 0 is no scry and does not trigger it (CR 701.22b). The scry's number and the cards looked at are the
/// trigger's `data` ("for each card looked at while scrying this way"). Arwen Undómiel, Chance-Met Elves and
/// Council's Deliberation print it.
pub fn you_scry(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::Scried { player, count, looked } if player == me.controller => {
            Some(Fired { subject: Target::None, data: [count as u32, looked as u32] })
        }
        _ => None,
    }
}

/// "Whenever you surveil" (CR 701.25, 603.2): its controller surveilled, once the surveil was complete (CR 701.25d).
/// Surveil 0 does not trigger it (CR 701.25c). Its `data` as `you_scry`'s. Dimir Spybug, Blood Operative and Copy
/// Catchers print it.
pub fn you_surveil(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::Surveilled { player, count, looked } if player == me.controller => {
            Some(Fired { subject: Target::None, data: [count as u32, looked as u32] })
        }
        _ => None,
    }
}

/// "Whenever you scry or surveil" (CR 701.22, 701.25): either one, each scry and each surveil on its own. Planetarium of
/// Wan Shi Tong, Denzilore Fatehold and Diviner of Victory print it.
pub fn you_scry_or_surveil(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    you_scry(q, me, event).or_else(|| you_surveil(q, me, event))
}

/// An activation (CR 602.2) as the event a "whenever ... activate(s) an ability" trigger sees: the object whose ability
/// it is, the player who activated it, and the ability. The object is read as it is, or as it last was if paying the cost
/// took it away (CR 113.7a, 608.2h). Only abilities that use the stack are activations triggers see (`EventMask::of`).
/// Ajani Unrelenting, Burning-Tree Shaman, Harsh Mentor, Immolation Shaman and Keral Keep Disciples ask it.
pub fn activation(q: &dyn Query, event: &Event) -> Option<(ObjectId, PlayerId, &'static ActivatedDef)> {
    let Event::AbilityActivated { source, ability, player, .. } = *event else { return None };
    q.activated_ability(ability).map(|def| (source, player, def))
}

/// "Whenever you activate a loyalty ability [of ...]" (CR 606.2, 603.2): the permanent whose loyalty ability the ability's
/// controller (`me`, CR 603.7d for a delayed one) activated, for a condition on it ("of a Chandra planeswalker"), read as it was when the costs were paid or
/// as it last was if paying them took it away. Ajani Unrelenting and Keral Keep Disciples print it.
pub fn loyalty_ability_you_activated(q: &dyn Query, me: &Asker, event: &Event) -> Option<ObjectId> {
    let (object, player, def) = activation(q, event)?;
    (def.is_loyalty_ability() && player == me.controller).then_some(object)
}

/// "Whenever you activate a loyalty ability": any permanent's, by this ability's controller (Ajani Unrelenting).
pub fn you_activate_a_loyalty_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    loyalty_ability_you_activated(q, me, event).and(Fired::PLAIN)
}

/// "Whenever an opponent activates an ability of an artifact, creature, or land [on the battlefield] that isn't a mana
/// ability, ... to that player" (CR 602.2, 603.2): a permanent's ability (one that functions on the battlefield, not
/// cycling from a hand or embalm from a graveyard), the permanent an artifact, creature or land as it was when the costs
/// were paid, a sacrificed one as it last was on the battlefield. The ability is about the activating player. Harsh
/// Mentor and Immolation Shaman print it.
pub fn opponent_activates_ability_of_artifact_creature_or_land(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (object, player, def) = activation(q, event)?;
    let types = q.chars(object);
    let permanent = def.from == ZoneKind::Battlefield
        && (types.is(CardType::Artifact) || types.is(CardType::Creature) || types.is(CardType::Land));
    let opponent = q.state().opponents(me.controller).any(|p| p == player);
    if permanent && opponent && !def.is_mana_ability() { Fired::about(Target::Player(player)) } else { None }
}

/// The ability an activation put on the stack (CR 602.2a), as a `Target::Ability` naming it there, for "copy that ability"
/// (`Game::copy_ability`): the topmost activated ability of that source and ability, which, as triggers are collected the
/// moment the activation is complete (CR 602.2b, 601.2i, 603.2), is the one just activated. Rings of Brighthearth, Kurkesh,
/// Illusionist's Bracers, Battlemage's Bracers and Locus of Enlightenment copy it.
pub fn activated_ability_on_stack(q: &dyn Query, event: &Event) -> Option<Target> {
    let Event::AbilityActivated { source, ability, .. } = *event else { return None };
    q.state()
        .stack
        .iter()
        .rev()
        .find(|i| i.kind == StackKind::Activated && i.object == source && i.ability == ability)
        .map(|i| Target::Ability(i.serial))
}

/// "Whenever you activate an ability that isn't a mana ability" (CR 602.2, 605.1a, 603.2): any ability this ability's
/// controller activates that uses the stack, wherever its source is, about that ability on the stack (to copy it,
/// `activated_ability_on_stack`). Locus of Enlightenment prints it, and Rings of Brighthearth as "if it isn't a mana ability".
pub fn you_activate_a_nonmana_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (_, player, def) = activation(q, event)?;
    if player != me.controller || def.is_mana_ability() {
        return None;
    }
    Fired::about(activated_ability_on_stack(q, event)?)
}

// --- The triggers, each with the events it listens to: `triggered(THIS_ENTERS, effect, text)` ---------------------

pub const YOU_ACTIVATE_A_NONMANA_ABILITY: Trigger = when_asked(EventMask::ABILITY_ACTIVATED, you_activate_a_nonmana_ability);

pub const THIS_ENTERS: Trigger = when(EventMask::ZONE_CHANGE, this_enters);
pub const THIS_ENTERS_IF_CAST: Trigger = when(EventMask::ZONE_CHANGE, this_enters_if_cast);
pub const LANDFALL: Trigger = when_asked(EventMask::ZONE_CHANGE, landfall);
pub const YOU_UNLOCK_THIS_RIGHT_DOOR: Trigger = when(EventMask::DOOR_UNLOCKED, you_unlock_this_right_door);
pub const YOU_UNLOCK_THIS_LEFT_DOOR: Trigger = when(EventMask::DOOR_UNLOCKED, you_unlock_this_left_door);
pub const YOU_SCRY: Trigger = when_asked(EventMask::SCRIED, you_scry);
pub const YOU_SURVEIL: Trigger = when_asked(EventMask::SURVEILLED, you_surveil);
pub const YOU_SCRY_OR_SURVEIL: Trigger = when_asked(EventMask(EventMask::SCRIED.0 | EventMask::SURVEILLED.0), you_scry_or_surveil);
pub const YOU_DRAW_A_CARD: Trigger = when_asked(EventMask::DREW, you_draw_a_card);
pub const AN_OPPONENT_DRAWS_A_CARD: Trigger = when_asked(EventMask::DREW, an_opponent_draws_a_card);
pub const YOU_GAIN_LIFE: Trigger = when_asked(EventMask::LIFE_CHANGE, you_gain_life);
/// "When you cast this spell": functions from the stack (CR 113.6k, `when_this_spell_is_cast`).
pub const CAST_THIS_SPELL: Trigger = when_this_spell_is_cast(cast_this_spell);

/// "When this card is put into a graveyard from anywhere" (CR 603.6c): its own move into a graveyard, from any zone, which
/// can't trigger from the battlefield, so it functions from the graveyard it has just reached (CR 113.6k) - and as "from
/// anywhere" it is never a leaves-the-battlefield ability, even from the battlefield (CR 603.6c, `read_zone_words`).
/// Emrakul, the Aeons Torn and Worldspine Wurm print it.
pub const PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE: Trigger =
    when(EventMask::ZONE_CHANGE, put_into_a_graveyard_from_anywhere).functions_in(IN_A_GRAVEYARD);

fn put_into_a_graveyard_from_anywhere(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Graveyard) => Fired::PLAIN,
        _ => None,
    }
}
pub const MAGECRAFT: Trigger = when_asked(EventMask(EventMask::SPELL_CAST.0 | EventMask::COPIED.0), you_cast_or_copy_instant_or_sorcery);
pub const THIS_ATTACKS_OR_BLOCKS: Trigger = when(EventMask(EventMask::ATTACKS.0 | EventMask::BLOCKS.0), this_attacks_or_blocks);
pub const THIS_DEALT_DAMAGE: Trigger = when(EventMask::DAMAGE, this_dealt_damage);
pub const DEALS_COMBAT_DAMAGE_TO_A_CREATURE: Trigger = when(EventMask::DAMAGE, deals_combat_damage_to_a_creature);
pub const THIS_ATTACKS: Trigger = when(EventMask::ATTACKS, this_attacks);
pub const THIS_ATTACKS_ALONE: Trigger = when(EventMask::ATTACKS, this_attacks_alone);
pub const CREATURE_YOU_CONTROL_ATTACKS_ALONE: Trigger = when_asked(EventMask::ATTACKS, creature_you_control_attacks_alone);
pub const EQUIPPED_CREATURE_ATTACKS_ALONE: Trigger = when(EventMask::ATTACKS, equipped_creature_attacks_alone);

/// "Whenever a player casts their second spell each turn" (CR 601.2i, 603.2): any player's, counted from the turn's start
/// (`Event::SpellCast::nth`), so the spells cast before the permanent arrived count too. Ledger Shredder and Hearthborn
/// Battler print it.
fn a_player_casts_their_second_spell(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { nth: 2, .. } => Fired::PLAIN,
        _ => None,
    }
}

pub const A_PLAYER_CASTS_THEIR_SECOND_SPELL: Trigger = when(EventMask::SPELL_CAST, a_player_casts_their_second_spell);
pub const THIS_ENTERS_OR_ATTACKS: Trigger = when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::ATTACKS.0), this_enters_or_attacks);
pub const THIS_ATTACKS_DEFENDER: Trigger = when(EventMask::ATTACKS, this_attacks_defender);
pub const EQUIPPED_CREATURE_ATTACKS: Trigger = when(EventMask::ATTACKS, equipped_creature_attacks);
pub const EQUIPPED_CREATURE_ATTACKS_SUBJECT: Trigger = when(EventMask::ATTACKS, equipped_creature_attacks_subject);
pub const EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER: Trigger =
    when(EventMask::DAMAGE, equipped_creature_deals_combat_damage_to_player);
pub const ENCHANTED_PLAYER_ATTACKED: Trigger = when(EventMask::ATTACKS, enchanted_player_attacked);
pub const YOU_ATTACK: Trigger = when_asked(EventMask::ATTACKERS_DECLARED, you_attack);
pub const INSTANT_OR_SORCERY_YOU_CAST: Trigger = when_asked(EventMask::SPELL_CAST, you_cast_instant_or_sorcery);
pub const YOU_CAST_A_SPELL: Trigger = when_asked(EventMask::SPELL_CAST, you_cast_a_spell);
pub const NONCREATURE_SPELL_YOU_CAST: Trigger = when_asked(EventMask::SPELL_CAST, noncreature_spell_you_cast);
pub const YOU_CAST_A_SPELL_FROM_EXILE: Trigger = when_asked(EventMask::SPELL_CAST, you_cast_a_spell_from_exile);
pub const THIS_DIES: Trigger = when(EventMask::ZONE_CHANGE, this_dies);
pub const THIS_LEAVES: Trigger = when(EventMask::ZONE_CHANGE, this_leaves);
pub const YOU_DRAW_YOUR_SECOND_CARD: Trigger = when_asked(EventMask::DREW, you_draw_your_second_card);
pub const YOUR_UPKEEP: Trigger = when_asked(EventMask::STEP_BEGAN, your_upkeep);
pub const YOUR_DRAW_STEP: Trigger = when_asked(EventMask::STEP_BEGAN, your_draw_step);
pub const YOUR_FIRST_MAIN_PHASE: Trigger = when_asked(EventMask::STEP_BEGAN, your_first_main_phase);
pub const YOUR_SECOND_MAIN_PHASE: Trigger = when_asked(EventMask::STEP_BEGAN, your_second_main_phase);
pub const EACH_OPPONENTS_UPKEEP: Trigger = when_asked(EventMask::STEP_BEGAN, each_opponents_upkeep);
pub const BEGINNING_OF_YOUR_COMBAT: Trigger = when_asked(EventMask::STEP_BEGAN, beginning_of_your_combat);
pub const YOUR_END_STEP: Trigger = when_asked(EventMask::STEP_BEGAN, your_end_step);
pub const EACH_UPKEEP: Trigger = when(EventMask::STEP_BEGAN, each_upkeep);
pub const EACH_END_STEP: Trigger = when(EventMask::STEP_BEGAN, each_end_step);
pub const END_OF_COMBAT: Trigger = when(EventMask::STEP_BEGAN, end_of_combat);
pub const CREATURE_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, creature_you_control_enters);
pub const CREATURE_WITH_POWER_FOUR_OR_MORE_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, creature_with_power_four_or_more_you_control_enters);
pub const CREATURE_YOU_CONTROL_ATTACKS: Trigger = when_asked(EventMask::ATTACKS, creature_you_control_attacks);
pub const DRAGON_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, dragon_you_control_enters);
pub const OPPONENTS_CREATURE_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, opponents_creature_enters);
pub const LEGENDARY_CREATURE_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, legendary_creature_you_control_enters);
pub const ARTIFACT_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, artifact_you_control_enters);
pub const ANOTHER_CREATURE_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, another_creature_you_control_enters);
pub const CREATURE_DIES: Trigger = when(EventMask::ZONE_CHANGE, creature_dies);

/// A different creature dies, using the characteristics it last had on the battlefield.
pub const ANOTHER_CREATURE_DIES: Trigger = when(EventMask::ZONE_CHANGE, another_creature_dies);
pub const CREATURE_YOU_CONTROL_DIES: Trigger = when_asked(EventMask::ZONE_CHANGE, creature_you_control_dies);
pub const ARTIFACT_SPELL_YOU_CAST: Trigger = when_asked(EventMask::SPELL_CAST, artifact_spell_you_cast);
pub const CREATURE_SPELL_YOU_CAST: Trigger = when_asked(EventMask::SPELL_CAST, creature_spell_you_cast);
pub const DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT: Trigger = when(EventMask::DAMAGE, deals_combat_damage_to_player_subject);
pub const DEALS_COMBAT_DAMAGE_TO_PLAYER: Trigger = when(EventMask::DAMAGE, deals_combat_damage_to_player);
pub const OPPONENT_ACTIVATES_ABILITY_OF_ARTIFACT_CREATURE_OR_LAND: Trigger =
    when_asked(EventMask::ABILITY_ACTIVATED, opponent_activates_ability_of_artifact_creature_or_land);
pub const YOU_ACTIVATE_A_LOYALTY_ABILITY: Trigger = when_asked(EventMask::ABILITY_ACTIVATED, you_activate_a_loyalty_ability);

/// "Whenever this creature transforms into [this face]" (CR 701.27e): the ability is that face's, so it is there to see
/// only once the permanent has turned over to it (CR 603.2, 603.6). A permanent that enters with this face up has not
/// transformed (CR 712.14). Ashling, Rimebound, Revealing Eye and Sygg, Wanderbrine Shield print it.
pub fn this_transforms(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Transformed { object } if *object == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever this creature enters or transforms into [this face]": either event (CR 603.2). Ashling, Rekindled and Sygg,
/// Wanderwine Wisdom print it.
pub fn this_enters_or_transforms(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    this_enters(q, source, event).or_else(|| this_transforms(q, source, event))
}

/// "When this permanent enters or leaves the battlefield" (CR 113.2c, 603.2, 603.6a, 603.6c): one triggered ability with two
/// conditions, its own entering the battlefield or its own leaving it, which looks back for the leaving alone (CR 603.10a;
/// `CardTable::looks_back` decides it per event). Cryogen Relic, Sewer-veillance Cam and Experimental Synthesizer print it. The
/// `listens` mask is `ZONE_CHANGE`.
pub fn this_enters_or_leaves(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    this_enters(q, source, event).or_else(|| this_leaves(q, source, event))
}

pub const THIS_ENTERS_OR_LEAVES: Trigger = when(EventMask::ZONE_CHANGE, this_enters_or_leaves);
pub const THIS_TRANSFORMS: Trigger = when(EventMask::TRANSFORMED, this_transforms);
pub const THIS_ENTERS_OR_TRANSFORMS: Trigger = when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::TRANSFORMED.0), this_enters_or_transforms);

/// "Whenever this creature deals combat damage to a player or planeswalker": one triggering for each player or
/// planeswalker it dealt damage to (CR 603.2c), judged as the damage is dealt (CR 510.2). Psychic Frog prints it, and Sygg,
/// Wanderwine Wisdom grants it.
pub fn deals_combat_damage_to_player_or_planeswalker(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: dealer, target, amount, kind, .. } if kind.is_combat() && *dealer == source && *amount > 0 => match target {
            Target::Player(_) => Fired::PLAIN,
            Target::Object(id) if q.chars(*id).is(CardType::Planeswalker) => Fired::PLAIN,
            _ => None,
        },
        _ => None,
    }
}

pub const DEALS_COMBAT_DAMAGE_TO_PLAYER_OR_PLANESWALKER: Trigger =
    when(EventMask::DAMAGE, deals_combat_damage_to_player_or_planeswalker);
