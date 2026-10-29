//! Shared mechanics (docs/engine-design.md, "Layers"): code that is the same across cards and differs only in
//! their data. Built from `cantrip-api` alone. Admission is CLAUDE.md's test: a second printed card would use it
//! exactly as written.

pub mod adapt;
pub mod affinity;
pub mod amass;
pub mod assist;
pub mod attach;
pub mod bargain;
pub mod behold;
pub mod blight;
pub mod bestow;
pub mod boast;
pub mod battle_cry;
pub mod backup;
pub mod beam_me_up;
pub mod dash;
pub mod discover;
pub mod combat;
pub mod companion;
pub mod copy;
pub mod cumulative_upkeep;
pub mod channel;
pub mod class;
pub mod choose;
pub mod casualty;
pub mod cipher;
pub mod chosen_type;
pub mod hand;
pub mod players;
pub mod history;
pub mod library;
pub mod impending;
pub mod firebending;
pub mod flashback;
pub mod harmonize;
pub mod flurry;
pub mod gift;
pub mod goad;
pub mod goyf;
pub mod harness;
pub mod hexproof_from;
pub mod job_select;
pub mod hideaway_play;
pub mod cleave;
pub mod craft;
pub mod crew;
pub mod devoid;
pub mod double;
pub mod devotion;
pub mod domain;
pub mod dredge;
pub mod earthbend;
pub mod echo;
pub mod empower;
pub mod enduring;
pub mod enlist;
pub mod escalate;
pub mod fading;
pub mod ferocious;
pub mod fight;
pub mod escape;
pub mod eternalize;
pub mod exert;
pub mod evoke;
pub mod for_mirrodin;
pub mod delayed;
pub mod effects;
pub mod foretell;
pub mod freerunning;
pub mod kicker;
pub mod learn;
pub mod level_up;
pub mod lands;
pub mod land_until_cast;
pub mod mana;
pub mod manifest_dread;
pub mod cloak;
pub mod disguise;
pub mod megamorph;
pub mod morph;
pub mod mayhem;
pub mod max_speed;
pub mod miracle;
pub mod melee;
pub mod mentor;
pub mod myriad;
pub mod extort;
pub mod evolve;
pub mod mobilize;
pub mod next_creature_counter;
pub mod ninjutsu;
pub mod offspring;
pub mod outlaw;
pub mod outside;
pub mod persist;
pub mod riot;
pub mod unleash;
pub mod embalm;
pub(crate) mod copy_from_graveyard;
pub mod undying;
pub(crate) mod returns_with_counter;
pub mod overload;
pub mod paradigm;
pub mod plot;
pub mod power_up;
pub mod prepare;
pub mod prototype;
pub mod prevention;
pub mod protection;
pub mod ravenous;
pub mod recruit;
pub mod reconfigure;
pub mod regenerate;
pub mod saddle;
pub mod select;
pub mod replicate;
pub mod retrace;
pub mod ripple;
pub mod replacements;
pub mod statics;
pub mod station;
pub mod storied;
pub mod storm;
pub mod spree;
pub mod start_your_engines;
pub mod tiered;
pub mod sneak;
pub mod solved;
pub mod spectacle;
pub mod surge;
pub mod spent;
pub mod suspend;
pub mod teamwork;
pub mod typecycling;
pub mod scavenge;
pub mod transmute;
pub mod transfigure;
pub mod unearth;
pub mod triggers;
pub mod warp;
pub mod weaken_attackers;
pub mod web_slinging;
pub mod targets;
pub mod text_change;
pub mod tokens;
pub mod training;
pub mod modular;
pub mod outlast;
pub mod increment;
pub mod ingest;
pub mod provoke;
pub mod flanking;
pub mod dethrone;
pub mod sunburst;
pub mod untap;
pub mod vote;

use cantrip_api::*;

/// "Activate only during your turn" (CR 602.5b), and "if it's your turn" on an alternative cost (CR 118.9): the
/// ability's or spell's controller is the active player. Wishclaw Talisman, Vivi Ornitier, Sundial of the Infinite and
/// Mine Collapse print it.
pub fn your_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.active == asker.controller
}

/// A player chooses `count` cards of their hand (all of it if they have fewer) and discards them together
/// (CR 701.9). Asked even when the choice is forced, except of an empty hand.
pub fn discard_chosen(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
    let chosen = choose_from_hand(g, player, source, count)?;
    g.discard_together(&chosen);
    Ok(chosen)
}

/// The cards a player chooses from their own hand for a discard, without discarding them yet.
pub fn choose_from_hand(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: u32) -> Result<Vec<ObjectId>, Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    choose::cards(g, player, source, Purpose::Discard, hand, count, false)
}

/// "Each [of these players] discards a card" (CR 701.9, 101.4): in the order given (turn order from the active player,
/// `apnap()`) each player with a card in hand chooses one (nothing is asked of an empty hand), then every chosen card is
/// discarded together. The cards discarded. Liliana of the Veil (each player) and Aclazotz, Deepest Betrayal (each
/// opponent) print it.
pub fn each_discards_a_card(g: &mut dyn Game, players: &[PlayerId], source: ObjectId) -> Result<Vec<ObjectId>, Suspend> {
    let mut chosen = Vec::new();
    for &player in players {
        if !crate::players::reached(g, player) {
            continue;
        }
        chosen.extend(choose_from_hand(g, player, source, 1)?);
    }
    Ok(g.discard_together(&chosen))
}

/// "[Each of `players`] discards two cards unless they discard [a quality of card]" (CR 608.2d, 701.9, 101.4): in the order
/// given (turn order from the active player, `apnap()`), each player still in the game chooses; then every chosen card is
/// discarded together, and the cards discarded are returned. With a card of the quality in hand a player chooses between
/// discarding one such card and discarding two cards of any kind (the card's `labels` 0 and 1 name the two answers, in that
/// order); a card of the quality may be one of the two. With none in hand nothing is offered but the two cards.
///
/// The one card pays the "unless", a cost that specifies a characteristic of it (CR 118.12a): no replacement that would put
/// it into a hidden zone unrevealed is offered (CR 701.9c, `Game::discard_paying`). The two cards specify none. Steamcore
/// Scholar, Winternight Stories and Bandit's Talent print it.
pub fn discard_two_unless_discarding(
    g: &mut dyn Game,
    players: &[PlayerId],
    source: ObjectId,
    qualifies: fn(&Chars) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let (mut chosen, mut paying) = (Vec::new(), Vec::new());
    for &player in players {
        if !players::reached(g, player) {
            continue;
        }
        let hand = g.state().zone(Zone::hand(player)).to_vec();
        let qualifying: Vec<ObjectId> = hand.iter().copied().filter(|&c| qualifies(&g.chars(c))).collect();
        if !qualifying.is_empty() {
            let card = g.obj(source).card;
            if choose::one_of(g, player, source, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0 {
                let one = choose::cards_from(g, player, source, Purpose::Discard, hand, qualifying, 1, false)?;
                paying.extend_from_slice(&one);
                chosen.extend(one);
                continue;
            }
        }
        chosen.extend(choose_from_hand(g, player, source, 2)?);
    }
    Ok(g.discard_paying(&chosen, &paying))
}

/// "[Player] discards `count` cards at random" (CR 701.9b; Hymn to Tourach, Mind Twist): the hand put in a random order
/// (Fisher-Yates, as the C# engine's `Randomize`) and the first `count` taken; a smaller hand is discarded whole. They
/// leave together, as one discard (CR 701.9a).
pub fn discard_at_random(g: &mut dyn Game, player: PlayerId, count: usize) {
    let mut hand = g.state().zone(Zone::hand(player)).to_vec();
    let count = hand.len().min(count);
    if count == 0 {
        return;
    }
    for i in (1..hand.len()).rev() {
        let j = g.random(i as u32 + 1) as usize;
        hand.swap(i, j);
    }
    g.discard_together(&hand[..count]);
}

/// "[Player] sacrifices a [kind]" (CR 701.21): chosen by that player when there is a choice, taken when there is not.
pub fn sacrifice_chosen(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: u8,
    kind: fn(&Chars) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let eligible: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| kind(&g.chars(id))).collect();
    sacrifice_one_of(g, player, source, count, eligible)
}

/// "Each opponent sacrifices `count` [kind] of their choice" (CR 701.21a, 101.4): in turn order from the active player,
/// each opponent chooses when there is a choice, all on the board as it is before anything is sacrificed, and then every
/// chosen permanent is sacrificed at once - so a later opponent's choice can't depend on an earlier one's sacrifice, and
/// "whenever a creature dies" sees them die together. `kind` is judged on the object as well as its characteristics, so it
/// can ask whether a permanent is a token. Portal to Phyrexia (three creatures), Sheoldred's Edict (a nontoken creature, a
/// creature token, a planeswalker) and Grave Pact ("each other player sacrifices a creature") print it.
pub fn each_opponent_sacrifices(
    g: &mut dyn Game,
    controller: PlayerId,
    source: ObjectId,
    count: u8,
    kind: fn(&dyn Query, ObjectId) -> bool,
) -> Result<(), Suspend> {
    each_opponent_sacrifices_choices(g, controller, source, count, kind).map(|_| ())
}

/// As `each_opponent_sacrifices`, returning who chose what (the old objects): an opponent missing from it sacrificed nothing,
/// having no permanent of the kind or being unable to be made to (CR 101.2), which "each opponent who can't" asks. Momentum
/// Breaker ("each opponent who can't discards a card") and Skull Storm ("each opponent who can't loses half their life")
/// print it.
pub fn each_opponent_sacrifices_choices(
    g: &mut dyn Game,
    controller: PlayerId,
    source: ObjectId,
    count: u8,
    kind: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<(PlayerId, ObjectId)>, Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().filter(|&p| p != controller).collect();
    sacrifice_together(g, &players, source, count, kind)
}

/// "Each player sacrifices `count` [kind] of their choice" (CR 701.21a, 101.4): as `each_opponent_sacrifices` but for every
/// player, the controller included, in turn order from the active player. Returns who chose what (the old objects), so that
/// "if you sacrificed a creature this way" can look at it. Garruk, Veiled Butcher's -2 and Innocent Blood print it.
pub fn each_player_sacrifices(
    g: &mut dyn Game,
    source: ObjectId,
    count: u8,
    kind: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<(PlayerId, ObjectId)>, Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    sacrifice_together(g, &players, source, count, kind)
}

/// "Each opponent chooses [one permanent of each of `kinds`] from among the [`among`] permanents they control, then
/// sacrifices the rest" (CR 701.21a, 101.4): each opponent in turn order keeps one permanent of each kind, asked kind by
/// kind and in view of all their `among` permanents, unless they control no more than one of that kind. A permanent with
/// several of the types may be chosen for several (the rulings), and counts as kept for all of them. Everything an opponent
/// controls among `among` that was not kept is sacrificed, all of it together, after every opponent has chosen. Ajani,
/// Nacatl Pariah's back face (the artifact, creature, enchantment and planeswalker among their nonland permanents) and
/// Liliana, Dreadhorde General's -9 (each permanent type) print it.
pub fn each_opponent_keeps_one_of_each_type(
    g: &mut dyn Game,
    controller: PlayerId,
    source: ObjectId,
    kinds: &[CardType],
    among: fn(&Chars) -> bool,
) -> Result<(), Suspend> {
    let mut doomed = Vec::new();
    for who in g.state().apnap().collect::<Vec<_>>() {
        if who == controller || !crate::players::reached(g, who) {
            continue;
        }
        let theirs: Vec<ObjectId> = g.permanents_of(who).into_iter().filter(|&id| among(&g.chars(id))).collect();
        let mut kept: Vec<ObjectId> = Vec::new();
        for &kind in kinds {
            let eligible: Vec<ObjectId> = theirs.iter().copied().filter(|&id| g.chars(id).is(kind)).collect();
            if eligible.len() <= 1 {
                kept.extend(eligible);
                continue;
            }
            // Each question names the type it is for: one permanent may be offered under several types.
            let word = match kind {
                CardType::Artifact => "artifact",
                CardType::Battle => "battle",
                CardType::Creature => "creature",
                CardType::Enchantment => "enchantment",
                CardType::Land => "land",
                CardType::Planeswalker => "planeswalker",
                _ => "permanent",
            };
            kept.extend(choose::cards_from_kind(g, who, source, Purpose::Keep, word, theirs.clone(), eligible, 1, false)?);
        }
        doomed.extend(theirs.into_iter().filter(|id| !kept.contains(id)).map(|id| (who, id)));
    }
    g.sacrifice_each(&doomed);
    Ok(())
}

/// Each of `players` in the order given chooses what they sacrifice, all on the board as it is before anything is
/// sacrificed, and then the choices are sacrificed at once (CR 101.4, 701.21a). Returns the choices with who made them.
fn sacrifice_together(
    g: &mut dyn Game,
    players: &[PlayerId],
    source: ObjectId,
    count: u8,
    kind: fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<(PlayerId, ObjectId)>, Suspend> {
    let mut chosen = Vec::new();
    for &who in players {
        if !crate::players::reached(g, who) {
            continue;
        }
        let eligible: Vec<ObjectId> = g.permanents_of(who).into_iter().filter(|&id| kind(&*g, id)).collect();
        chosen.extend(chosen_to_sacrifice(g, who, source, count, eligible)?.into_iter().map(|id| (who, id)));
    }
    g.sacrifice_each(&chosen);
    Ok(chosen)
}

/// "[Player] sacrifices `count` of `eligible`" (CR 701.21): the permanents the effect allows are decided by the caller (a
/// quality of the group, such as the greatest mana value among them), chosen by that player when there is a choice and all
/// of them taken when there are no more than `count`. `sacrifice_chosen` filters by kind; Break Under Pressure's "with the
/// greatest mana value among creatures and planeswalkers they control" cannot be a kind.
pub fn sacrifice_one_of(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: u8, eligible: Vec<ObjectId>) -> Result<Vec<ObjectId>, Suspend> {
    let chosen = chosen_to_sacrifice(g, player, source, count, eligible)?;
    Ok(g.sacrifice(player, &chosen))
}

/// What `player` will sacrifice of `eligible`: of those they can sacrifice (`Game::can_sacrifice`, CR 701.21a, 101.2 - "you can't try
/// to sacrifice [it], fail, and therefore ignore the effect"), all of them when there are no more than `count`, otherwise
/// `count` of their choice. Nothing is sacrificed yet, so that several players' choices can be sacrificed together (CR 101.4).
fn chosen_to_sacrifice(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: u8, eligible: Vec<ObjectId>) -> Result<Vec<ObjectId>, Suspend> {
    let eligible: Vec<ObjectId> = eligible.into_iter().filter(|&id| g.can_sacrifice(player, id)).collect();
    if eligible.len() <= count as usize {
        return Ok(eligible);
    }
    choose::cards(g, player, source, Purpose::Sacrifice, eligible, u32::from(count), false)
}

/// "An untapped creature you control" as the permanents a cost taps (CR 118.3): `TAP_AN_UNTAPPED_CREATURE` taps one, Group Project's
/// flashback (`tap_untapped(3, ..)`) three. A creature that came under your control this turn counts (CR 302.6 limits only {T}).
pub fn untapped_creature_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.chars(id).is(CardType::Creature)
}

/// Delirium (an ability word, CR 207.2c): four or more card types among the cards in `player`'s graveyard. Fear of Missing Out and
/// Unholy Heat read it; a static ability asks the same of a `LayerQuery` with `statics::itself_with_delirium`.
pub fn delirium(q: &dyn Query, player: PlayerId) -> bool {
    card_types_in_graveyard(q, player) >= 4
}

/// Spell mastery (an ability word, CR 207.2c): two or more instant and/or sorcery cards in `player`'s graveyard. Fiery Impulse reads
/// it as it resolves; Ghitu Lavarunner's static ability asks the same of a `LayerQuery` with `statics::spell_mastery`.
pub fn spell_mastery(q: &dyn Query, player: PlayerId) -> bool {
    q.state().zone(Zone::graveyard(player)).iter().filter(|&&c| q.chars(c).is(CardType::Instant) || q.chars(c).is(CardType::Sorcery)).count() >= 2
}

/// How many creature cards are in `player`'s graveyard (CR 404.1, 109.5): cards only, never tokens, counted as it is asked.
/// Grist, the Hunger Tide, Kraul Harpooner and Golgari Grave Troll count them for a number; Arnim Zola, Bio-Fanatic, HYDRA
/// Troopers and Klaw, Sonic Subjugator ask it against a limit.
pub fn creature_cards_in_graveyard(q: &dyn Query, player: PlayerId) -> usize {
    q.state().zone(Zone::graveyard(player)).iter().filter(|&&card| q.chars(card).is(CardType::Creature)).count()
}

/// How many card types are among the cards in `player`'s graveyard (CR 205.2a, 702.138a): each type counted once however many
/// cards have it, a card with several types giving each of them. Delirium reads it against four; Omnivorous Flytrap's "six or
/// more card types among cards in your graveyard" reads it again against six as its ability resolves.
pub fn card_types_in_graveyard(q: &dyn Query, player: PlayerId) -> usize {
    let mut types = TypeSet::NONE;
    for &id in q.state().zone(Zone::graveyard(player)) {
        for t in q.chars(id).types.iter() {
            types = types.with(t);
        }
    }
    types.iter().count()
}

/// "As an additional cost to cast this spell, pay {4} or sacrifice an artifact or creature" (CR 601.2b, 118.8): the two ways
/// of paying it, for a card's `additional_choice`. Deadly Precision and Stir Up Trouble print it.
pub static PAY_FOUR_OR_SACRIFICE_AN_ARTIFACT_OR_CREATURE: [ExtraCost; 2] = [
    ExtraCost { text: "Pay {4}", cost: Cost { mana: mana("{4}"), ..Cost::NONE } },
    ExtraCost { text: "Sacrifice an artifact or creature", cost: Cost::sacrifice(1, targets::filters::artifact_or_creature) },
];

/// Threshold (ability word, CR 207.2c): seven or more cards in `player`'s graveyard, counted as the question is asked.
/// Cabal Ritual's "threshold" (the resolving spell is on the stack, so not counted), Eye of Jace and Tersa Lightshatter read it.
pub fn threshold(q: &dyn Query, player: PlayerId) -> bool {
    q.state().zone(Zone::graveyard(player)).len() >= 7
}

/// "Tap an untapped creature you control" as a cost (CR 118.3): one creature its payer chooses, summoning sick or not,
/// since it is not the {T} symbol of that creature (CR 107.5, 302.6). Earthcraft, Kyren Negotiations and Springleaf Drum
/// print it.
pub const TAP_AN_UNTAPPED_CREATURE: Cost = Cost::tap_untapped(1, untapped_creature_you_control);

/// "Tap another untapped creature you control" as a cost (CR 118.3): as `TAP_AN_UNTAPPED_CREATURE`, but never the ability's
/// own source, even when it is a creature ("another"). Station (CR 702.184a) and Tyvar, the Pummeler print it.
pub const TAP_ANOTHER_UNTAPPED_CREATURE: Cost = Cost::tap_untapped(1, another_untapped_creature_you_control);

fn another_untapped_creature_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && untapped_creature_you_control(q, asker, id)
}

/// "Target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its
/// mana cost" (CR 702.34a): its controller may cast it from the graveyard this turn for its mana cost, and it is exiled
/// instead of going anywhere else as it leaves the stack. Snapcaster Mage and Flashback print it.
pub fn grant_flashback(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(card) = item.target(0)
    {
        g.allow_play(card, item.controller, PlayGrant { expiry: Expiry::EndOfTurn, flashback: true, ..PlayGrant::CAST });
    }
    Ok(())
}

/// Connive (CR 701.50): draw a card, then discard a card; if a nonland card was discarded, put a +1/+1 counter on
/// the creature. A creature that has left the battlefield still connives, and takes no counter; its controller is
/// the one it had last (CR 608.2h).
pub fn connive(g: &mut dyn Game, creature: ObjectId, source: ObjectId) -> Result<(), Suspend> {
    connive_n(g, creature, source, 1)
}

/// Connive N (CR 701.50): draw N cards, then discard N (as many as the hand holds), together, and a +1/+1 counter
/// goes on the creature for each nonland card discarded, read where it went by its discard (`hand::discarded_nonland_card`:
/// one put into a hidden zone unrevealed is undefined, no nonland card, CR 701.9c). Raffine, Scheming Seer connives X; Ledger Shredder, 1.
pub fn connive_n(g: &mut dyn Game, creature: ObjectId, source: ObjectId, count: u32) -> Result<(), Suspend> {
    let player = g.obj(creature).controller;
    g.draw(player, count);
    let chosen = choose_from_hand(g, player, source, count)?;
    let discarded = g.discard_together(&chosen);
    let nonland = discarded.iter().filter(|&&c| hand::discarded_nonland_card(&*g, c)).count() as i16;
    if nonland > 0 && g.is_on_battlefield(creature) && g.obj(creature).became.is_none() {
        g.add_counters(creature, CounterKind::PlusOne, nonland);
    }
    Ok(())
}

/// "Exile [it], then return it to the battlefield transformed under [a player's] control" (CR 712.14a): Ajani,
/// Nacatl Pariah, Fable of the Mirror-Breaker and The Legend of Roku print it. Nothing if the permanent has left the
/// battlefield already, since the new object is not the one the ability means (CR 400.7).
pub fn exile_and_return_transformed(g: &mut dyn Game, permanent: ObjectId, under: PlayerId) -> Result<(), Suspend> {
    if g.obj(permanent).became.is_some() || !g.is_on_battlefield(permanent) {
        return Ok(());
    }
    // A token that is exiled is gone for good (CR 111.7): only a card comes back.
    let token = g.obj(permanent).token;
    let exiled = g.exile(permanent);
    if !token && g.obj(exiled).zone.is(ZoneKind::Exile) {
        g.put_onto_battlefield_transformed(exiled, under, false)?;
    }
    Ok(())
}

/// "Exile [these permanents], then return them to the battlefield under their owner's control": all exiled at once (CR 608.2c,
/// 101.4), then all returned at once, as new objects (CR 400.7). A token exiled this way ceases to exist and does not return
/// (CR 111.7). Ephemerate, Displacer Kitten, Skycoach Conductor, The Mind Stone and Gone Fishing print it.
pub fn exile_and_return(g: &mut dyn Game, permanents: &[ObjectId]) -> Result<(), Suspend> {
    exile_and_return_objects(g, permanents).map(|_| ())
}

/// `exile_and_return`, the new objects that came back, for a card that then does something to them: Essence Flux's "If it's
/// a Spirit, put a +1/+1 counter on it."
pub fn exile_and_return_objects(g: &mut dyn Game, permanents: &[ObjectId]) -> Result<Vec<ObjectId>, Suspend> {
    let returning: Vec<(ObjectId, PlayerId)> = g
        .exile_all(permanents)
        .into_iter()
        .filter(|&card| !g.obj(card).token && g.obj(card).zone.is(ZoneKind::Exile))
        .map(|card| (card, g.obj(card).owner))
        .collect();
    g.put_all_onto_battlefield(&returning)
}

/// Exiles the targeted card, if it is still a legal target.
pub fn exile_target(g: &mut dyn Game, item: &StackItem, slot: usize) -> bool {
    if !g.target_legal(item, slot) {
        return false;
    }
    if let Target::Object(id) = g.state().target(item, slot) {
        g.exile(id);
        return true;
    }
    false
}

/// Destroys the targeted permanent, if it is still a legal target (CR 701.8, 608.2b); the permanent, so a caller can go on
/// to read what it was (its controller, its power). Disenchant and Vindicate print it.
pub fn destroy_target(g: &mut dyn Game, item: &StackItem, slot: usize) -> Option<ObjectId> {
    let id = targets::object(g, item, slot)?;
    g.destroy(id);
    Some(id)
}

/// "Destroy target [thing]. It can't be regenerated" (CR 701.8, 701.19c): `destroy_target` that a regeneration shield does
/// not save. Snuff Out and Echoing Ruin print it.
pub fn destroy_target_no_regen(g: &mut dyn Game, item: &StackItem, slot: usize) -> Option<ObjectId> {
    let id = targets::object(g, item, slot)?;
    g.destroy_no_regen(id);
    Some(id)
}

/// Counters the targeted spell, if it is still a legal target (CR 701.6, 608.2b): Counterspell and Annul print it.
pub fn counter_target(g: &mut dyn Game, item: &StackItem, slot: usize) -> Option<ObjectId> {
    let spell = targets::object(g, item, slot)?;
    g.counter(spell);
    Some(spell)
}

/// Counters the targeted activated or triggered ability (CR 701.6, 115.1), if it is still a legal target (CR 608.2b): the
/// ability as it was on the stack, so a caller can go on to read whose it was and what its source is. Stifle, Tishana's
/// Tidebinder and Ertai Resurrected print it.
pub fn counter_target_ability(g: &mut dyn Game, item: &StackItem, slot: usize) -> Option<StackItem> {
    if slot >= item.target_count as usize || !g.target_legal(item, slot) {
        return None;
    }
    let Target::Ability(serial) = g.state().target(item, slot) else { return None };
    let at = g.state().stack.iter().position(|i| i.serial == serial)?;
    let countered = g.state().stack[at];
    g.counter_item(countered.object, at);
    Some(countered)
}

/// What "counter [it] unless [its controller] pays [cost]" asks the payer (CR 118.12a): ward's (CR 702.21a) and the
/// counterspells' that leave it to the spell's controller (Mana Leak, No More Lies).
pub use cantrip_api::intrinsic::COUNTERED_UNLESS_PAID;

/// "Counter target spell unless its controller pays `cost`" (CR 118.12, 701.6): the spell's controller is asked, and may
/// make mana for it while asked; the cost is judged as the effect resolves, so a card whose cost depends on the board
/// (Override's {1} per artifact) reads it first and passes it. Mana Leak and Miscalculation print it with a fixed cost.
pub fn counter_unless_pays(g: &mut dyn Game, item: &StackItem, slot: usize, cost: ManaCost) -> Result<Option<ObjectId>, Suspend> {
    let Some(spell) = targets::object(g, item, slot) else { return Ok(None) };
    let payer = g.controller(spell);
    if !g.may_pay(payer, item.object, spell, Cost { mana: cost, ..Cost::NONE }, &COUNTERED_UNLESS_PAID)? {
        g.counter(spell);
    }
    Ok(Some(spell))
}

/// Exiles the targeted permanent until `source` leaves the battlefield (CR 610.3, 400.7): a returning card is a new
/// object. The permanent that was exiled, if the target was still legal. Aang's Iceberg and Conclave Tribunal print it
/// (Portable Hole and Sheltered by Ghosts as well).
pub fn exile_target_until_leaves(g: &mut dyn Game, item: &StackItem, slot: usize) -> Option<ObjectId> {
    let id = targets::object(g, item, slot)?;
    g.exile_with(&[id], g.linked(item), true);
    Some(id)
}

/// "Exile each/all <permanents> ... until this leaves the battlefield" (CR 610.3, 400.7): every battlefield permanent
/// `keep` accepts, whoever controls it, chosen as the ability resolves and exiled together as one operation (CR 101.4).
/// Nothing is exiled at all if the source already left (CR 610.3b). Pinnacle Starcage and Temporary Lockdown print it.
pub fn exile_all_until_leaves(g: &mut dyn Game, item: &StackItem, mut keep: impl FnMut(&mut dyn Game, ObjectId) -> bool) {
    let on_battlefield: Vec<ObjectId> = g.state().battlefield().iter().copied().collect();
    let chosen: Vec<ObjectId> = on_battlefield.into_iter().filter(|&id| keep(&mut *g, id)).collect();
    g.exile_with(&chosen, g.linked(item), true);
}

/// The resolution of "exile target ... until this leaves the battlefield" as a triggered ability's effect: exiles the
/// slot 0 target (`exile_target_until_leaves`). It is not exiled at all if the source already left (CR 610.3b).
/// Portable Hole and Seam Rip use it.
pub fn exile_target_until_leaves_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target_until_leaves(g, item, 0);
    Ok(())
}

/// Airbend (CR 701.65a): exiles the resolving ability's legal targets together (`airbend_all`). Appa, Steadfast Guardian
/// and Aang, Swift Savior print it.
pub fn airbend(g: &mut dyn Game, item: &StackItem) {
    let targets: Vec<ObjectId> =
        (0..item.target_count as usize).filter(|&slot| g.target_legal(item, slot)).filter_map(|slot| g.state().target(item, slot).object()).collect();
    airbend_all(g, &targets);
}

/// Airbend (CR 701.65a) these objects, targeted or not: they are exiled together, and each card exiled this way may be cast
/// by its owner for {2} rather than its mana cost for as long as it stays exiled. A token or a copy is exiled and gone: only
/// a card can be cast again. Avatar's Wrath ("airbend all other creatures") and Monk Gyatso ("you may airbend that
/// creature") airbend what they name without targeting it; `airbend` is the targeted form.
pub fn airbend_all(g: &mut dyn Game, objects: &[ObjectId]) {
    for exiled in g.exile_all(objects) {
        let obj = *g.obj(exiled);
        if obj.zone.is(ZoneKind::Exile) && !obj.token && !obj.card_copy {
            g.allow_play(exiled, obj.owner, PlayGrant { instead: Some(mana("{2}")), ..PlayGrant::CAST });
        }
    }
}

/// "The cards in a player's hand" (CR 402.1). Beza, the Bounding Spring and Robber of the Rich compare it.
pub fn cards_in_hand(q: &dyn Query, player: PlayerId) -> usize {
    q.state().zone(Zone::hand(player)).len()
}

/// The zone a spell is cast from while its cost is worked out (CR 601.2f): the zone the card is in before it is cast,
/// or the one it was cast from once it is on the stack. Bilbo, Thief in the Night, Norman Osborn and Aven Interrupter
/// change what spells cast from certain zones cost.
pub fn zone_cast_from(q: &dyn Query, spell: ObjectId) -> Option<ZoneKind> {
    let obj = q.obj(spell);
    if obj.zone.is(ZoneKind::Stack) { obj.cast_from } else { Some(obj.zone.kind()) }
}

/// "The number of creatures [player] controls": every creature on the battlefield they control, read through the layers.
/// Ajani, Nacatl Pariah, Emeritus of Truce, Oath of Druids and Stadium Headliner count it exactly so.
pub fn creatures_you_control(q: &dyn Query, player: PlayerId) -> usize {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Creature)).count()
}

/// "The number of [subtype]s [player] controls" (CR 205.3, 109.4): every permanent of that subtype they control, read through
/// the layers, so a changeling counts for each creature type. Beorn the Fierce's Bears, The Notary Hobbits' Halflings and Gloom Ripper's Elves
/// are such counts.
pub fn permanents_with_subtype_you_control(q: &dyn Query, player: PlayerId, subtype: Subtype) -> usize {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(subtype)).count()
}

/// "The number of [subtype] cards in [player]'s graveyard" (CR 404.1, 205.3): cards only, never tokens, counted as it is
/// asked. Dawnhand Eulogist, Creakwood Safewright and Gloom Ripper ask it for Elves; Accumulate Wisdom, Combustion Technique
/// and Gran-Gran for Lessons.
pub fn subtype_cards_in_graveyard(q: &dyn Query, player: PlayerId, subtype: Subtype) -> usize {
    q.state().zone(Zone::graveyard(player)).iter().filter(|&&card| q.chars(card).has_subtype(subtype)).count()
}

/// "The number of lands [player] controls": every land on the battlefield they control, read through the layers. Beza, the
/// Bounding Spring, Consult the Star Charts, Fabled Passage, Pillage the Bog and Emeritus of Abundance count it exactly so.
pub fn lands_you_control(q: &dyn Query, player: PlayerId) -> usize {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Land)).count()
}

/// Deals damage to the target in `slot`, if it is still legal.
pub fn damage_target(g: &mut dyn Game, item: &StackItem, slot: usize, amount: u16) {
    if g.target_legal(item, slot) {
        g.damage(item.object, g.state().target(item, slot), amount);
    }
}

/// "[Source] deals `amount` damage to each of [up to N / one or two] targets" (CR 120.3, 608.2b): every target of the resolving
/// item that is still legal is dealt `amount` by the item's source, all together as one event. A target that has become
/// illegal is left out. Prismari Charm, Batroc the Leaper and Way of the Warlord print it.
pub fn damage_each_target(g: &mut dyn Game, item: &StackItem, amount: u16) {
    let hits: Vec<(ObjectId, Target, u16)> =
        (0..item.target_count as usize).filter(|&slot| g.target_legal(item, slot)).map(|slot| (item.object, g.state().target(item, slot), amount)).collect();
    g.damage_together(&hits);
}

/// Deals each target of the resolving item its own share of the damage it was divided into (CR 601.2d, 120.3): "N damage
/// divided as you choose among [targets]". The shares were fixed as the spell or ability was put on the stack, a target
/// that has since become illegal is left out with its share (CR 608.2b), and the rest is dealt together as one event.
/// Pyrokinesis, Fire Covenant, Fury and Shatterskull Smashing print it.
pub fn damage_divided(g: &mut dyn Game, item: &StackItem) {
    let hits: Vec<(ObjectId, Target, u16)> = (0..item.target_count as usize)
        .filter(|&slot| g.target_legal(item, slot) && g.state().share(item, slot) > 0)
        // Damage is dealt in a u16 for now: a share past it deals as much as that holds (the damage widening is W2's).
        .map(|slot| (item.object, g.state().target(item, slot), u16::try_from(g.state().share(item, slot)).unwrap_or(u16::MAX)))
        .collect();
    g.damage_together(&hits);
}

/// Puts on each target of the resolving item its own share of the counters it was divided into (CR 601.2d, 122.1):
/// "distribute N [kind] counters among [targets]". The shares were fixed as the spell or ability was put on the stack, a
/// target that has since become illegal is left out and its counters are lost (CR 608.2b). That each target gets at least
/// one is the announcement's rule (the core's `Divide`), not asked again here. The counters are put on all the targets at
/// once (CR 608.2, 101.4: one event, CR 603.2c). Court of Garenbrig and Elusive Otter's Grove's Bounty print it.
pub fn counters_divided(g: &mut dyn Game, item: &StackItem, kind: CounterKind) {
    let mut puts: Vec<(ObjectId, CounterKind, i16)> = Vec::new();
    for slot in 0..item.target_count as usize {
        if let Some(object) = targets::object(g, item, slot)
            && g.state().share(item, slot) > 0
        {
            puts.push((object, kind, i16::try_from(g.state().share(item, slot)).unwrap_or(i16::MAX)));
        }
    }
    g.add_counters_together(&puts);
}

/// A one-color choice (CR 105.1), asked of `player`.
pub fn choose_color(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<Color, Suspend> {
    choose_color_for(g, player, source, ColorUse::Any)
}

/// A one-color choice (CR 105.1) with what it is for said (`ColorUse`): "a color that benefits you" (Heraldic Banner, Sunken
/// Citadel) or the color of mana to add (Deathrite Shaman, Ashling Rekindled, Boommobile).
pub fn choose_color_for(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: ColorUse) -> Result<Color, Suspend> {
    // The core holds the answer to the question: exactly one of the five colors (`validate`).
    let Answer::Colors(colors) = g.ask(player, RequestKind::ChooseColors { source, choices: ColorSet(0b11111), count: 1, purpose })? else {
        unreachable!("the core answers a color choice with colors")
    };
    Ok(colors[0])
}

/// "`count` mana in any combination of [colors]" (CR 106.1b): a color for each mana, among `choices`, repeats allowed,
/// asked of `player` in one question. Manamorphose, Orcish Lumberjack, Relic of Sauron, Sunken Ruins, Vivi Ornitier,
/// Coalition Relic, Wizard's Rockets, Desolation of Smaug and Sarkhan, Fireblood ask it.
pub fn choose_colors(g: &mut dyn Game, player: PlayerId, source: ObjectId, choices: ColorSet, count: u8) -> Result<Vec<Color>, Suspend> {
    // The core holds the answer to the question: `count` colors, each among `choices` (`validate`).
    let Answer::Colors(colors) = g.ask(player, RequestKind::ChooseColors { source, choices, count, purpose: ColorUse::AddMana })? else {
        unreachable!("the core answers a color choice with colors")
    };
    Ok(colors)
}

/// `searcher` searches the library of `owner` (CR 701.23, 701.23f; the two are the same player for "search your
/// library") for up to `count` cards (exactly as many as can be found, when not `optional`) that pass `eligible`, asked as
/// `library::search` asks with the rule that says so: `select::up_to(count)` (any number for a `count` of `ANY_NUMBER`) or
/// `select::as_many_as_can(count)` of the cards `eligible` passes. A search for zero cards, as Settle the Wreckage can make,
/// finds nothing and is still a search. Kept for existing cards; new cards use `library::search` (or the
/// `library::find_to_*` wrappers) with a rule from `select`.
pub fn search_library(
    g: &mut dyn Game,
    searcher: PlayerId,
    owner: PlayerId,
    source: ObjectId,
    destination: SearchDestination,
    count: u32,
    optional: bool,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let library = library::searched_cards(g, owner);
    let can: Vec<ObjectId> = library.iter().copied().filter(|&c| eligible(g, c)).collect();
    let count = if count == 0 {
        select::Count::UpTo(0)
    } else if optional && count == ANY_NUMBER {
        select::ANY_NUMBER
    } else if optional {
        select::up_to(count)
    } else {
        select::as_many_as_can(count)
    };
    library::search(g, searcher, owner, source, destination, Selection::listed(count, &library, can))
}

/// The count of a search for "any number of [cards]" (`search_library`, `library::Find::ONE.up_to(ANY_NUMBER)`). Kept for
/// existing cards; new cards use `select::ANY_NUMBER`.
pub const ANY_NUMBER: u32 = u32::MAX;

/// The top `count` cards of a player's library, top first.
pub fn top_of_library(g: &dyn Query, player: PlayerId, count: usize) -> Vec<ObjectId> {
    g.state().zone(Zone::library(player)).iter().take(count).copied().collect()
}

/// "[It] gets +P/+T until end of turn" (CR 611.2): a continuous effect from the resolving ability's source.
pub fn pump(g: &mut dyn Game, source: ObjectId, creature: ObjectId, power: i16, toughness: i16) {
    if g.is_on_battlefield(creature) && g.obj(creature).became.is_none() {
        g.add_effect(source, creature, EffectKind::Pump(power, toughness), Duration::EndOfTurn);
    }
}

/// "[It] gains haste until end of turn" (CR 611.2, 702.10): a continuous effect from the resolving ability's source. Devastating Onslaught and Song of Totentanz print it.
pub fn haste_until_end_of_turn(g: &mut dyn Game, source: ObjectId, creature: ObjectId) {
    if !g.is_on_battlefield(creature) {
        return;
    }
    effects::gain_until_end_of_turn(g, source, creature, Keyword::Haste);
}

/// "[It] gains trample until end of turn" (CR 611.2, 702.19): a continuous effect from the resolving spell or ability's source.
/// Ancestral Anger, Berserk and Fists of Flame print it.
pub fn trample_until_end_of_turn(g: &mut dyn Game, source: ObjectId, creature: ObjectId) {
    effects::gain_until_end_of_turn(g, source, creature, Keyword::Trample);
}

/// "The number of artifacts you control": the artifacts `player` controls as it is read (a resolving spell's or ability's X is
/// counted once, as it resolves, CR 608.2h). Aerid Konstrari, Craterclaw Colossus, Dowsing Device, Fomori Vault, Override and Tolarian Academy print it, and Mox Opal and Inventor's Fair's metalcraft
/// count it against three. (Adaptive Omnitool and Cranial Plating count through the layers of a static ability, which is a `LayerQuery`, not a `Query`.)
pub fn artifacts_you_control(q: &dyn Query, player: PlayerId) -> usize {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).is(CardType::Artifact)).count()
}

/// Scry N (CR 701.22): look at the top N cards of your library, put any number on the bottom in any order and the
/// rest back on top in any order. Nothing is asked of an empty library. Undercity's Lost Well prints "scry 2".
///
/// The scry is then recorded (`Event::Scried`), for "whenever you scry": once the process is complete, even if some or
/// all of it was impossible, an empty library included (CR 701.22d). Scry 0 is no scry: nothing is looked at and no
/// event occurs (CR 701.22b). Every card that scries goes through here.
pub fn scry(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    if count == 0 {
        return Ok(());
    }
    let looked = top_of_library(g, player, count);
    if !looked.is_empty() {
        let ask = RequestKind::DistributeCards {
            source,
            cards: looked.clone(),
            destinations: vec![
                Destination { kind: DestinationKind::LibraryTop, min: 0, max: u8::MAX, eligible: looked.clone() },
                Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u8::MAX, eligible: looked.clone() },
            ],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
            g.arrange_library(player, &placements[0], &placements[1]);
        }
    }
    g.scried(player, count_u32(count), count_u32(looked.len()));
    Ok(())
}

/// Surveil N (CR 701.25): look at the top N cards of your library, put any number into your graveyard (together, their
/// owner arranging them as they arrive when the order can matter, CR 404.3) and the rest back on top in any order. Nothing is asked of an empty
/// library. Second card: Consider, Thought Monitor's surveil, any "surveil 2".
pub fn surveil(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    surveil_into_graveyard(g, player, source, count).map(|_| ())
}

/// `surveil`, saying which cards it put into the graveyard (the new objects, those that really arrived there: a
/// replacement effect may have sent one elsewhere) for text that goes on to speak of them ("if you put a card ... into
/// your graveyard this way"). Enlightened Confidant prints it.
///
/// The surveil is then recorded (`Event::Surveilled`), for "whenever you surveil": once the process is complete, even if
/// some or all of it was impossible, an empty library included (CR 701.25d). Surveil 0 is no surveil: nothing is looked
/// at and no event occurs (CR 701.25c). Every card that surveils goes through here.
pub fn surveil_into_graveyard(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
) -> Result<Vec<ObjectId>, Suspend> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let looked = top_of_library(g, player, count);
    let mut binned = Vec::new();
    if !looked.is_empty() {
        let ask = RequestKind::DistributeCards {
            source,
            cards: looked.clone(),
            destinations: vec![
                Destination { kind: DestinationKind::LibraryTop, min: 0, max: u8::MAX, eligible: looked.clone() },
                Destination { kind: DestinationKind::Graveyard, min: 0, max: u8::MAX, eligible: looked.clone() },
            ],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
            let moves: Vec<(ObjectId, Zone)> = placements[1].iter().map(|&c| (c, Zone::graveyard(player))).collect();
            let moved = g.move_together(&moves, MoveReason::Other);
            binned = moved.into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Graveyard)).collect();
            g.arrange_library(player, &placements[0], &[]);
        }
    }
    g.surveilled(player, count_u32(count), count_u32(looked.len()));
    Ok(binned)
}

/// What "becomes a P/T [colors] [subtypes] [types] creature with [keywords]" makes of a permanent (CR 205.1b, 613.1).
/// `colors` is `None` when the text names none ("becomes a 4/4 artifact creature"): the permanent keeps its own.
/// `subtypes` are the creature types the text names, read through the permanent's words (CR 612), and empty when it
/// names none; "with all creature types" gives it every one besides (layer 4). `types` are the card types the text names
/// beside Creature, `TypeSet::NONE` for a plain "creature": Inkmoth Nexus ("a 1/1 Phyrexian Blinkmoth artifact
/// creature"), Blinkmoth Nexus and Mishra's Factory print "artifact creature", and so do Firdoch Core and Iron Man Armor,
/// which are artifacts already.
pub struct Animation<'a> {
    pub power: i16,
    pub toughness: i16,
    pub colors: Option<ColorSet>,
    /// Card types added beside Creature (layer 4).
    pub types: TypeSet,
    /// Creature types added (layer 4): two for "Construct Hero" (Iron Man Armor) and "Phyrexian Blinkmoth" (Inkmoth
    /// Nexus), one for "Horror" (Restless Cottage).
    pub subtypes: &'a [Subtype],
    /// "with all creature types" (CR 205.3m): Mutavault, Soulstone Sanctuary.
    pub all_creature_types: bool,
    /// The text makes it "a [creature types] artifact creature" and does not say it is still what it was, so the creature
    /// types it names replace those it had, while its card types and other subtypes stay (CR 205.1b): Iron Man Armor ("a
    /// 0/0 Construct Hero artifact creature"), Chimeric Staff ("an X/X Construct artifact creature"). A text that says "It's
    /// still a land" (Inkmoth Nexus, Mishra's Factory) keeps all its subtypes and leaves this false.
    pub replaces_creature_types: bool,
    pub keywords: Keywords,
    /// A triggered or activated ability the permanent is given for the duration (CR 613.1f), if the text gives one.
    pub ability: Option<AbilityRef>,
}

/// "[Until end of turn,] this land becomes a P/T [colors] [subtypes] [types] creature with [keywords]. It's still a land",
/// for the `duration` the text gives (Great Hall of the Biblioplex has none: `Duration::Indefinite`): the creature type and
/// any other card types the text names are added (layer 4) with its subtypes or all creature types, the colors replace
/// its own (5), the keywords and the ability are granted (6) and the base power and toughness set (7b), so counters and
/// pumps count on top. Celestial Colonnade and Creeping Tar Pit print it until end of turn, Firdoch Core names no colors
/// or creature type and leaves those two out, Inkmoth Nexus adds the artifact type and two creature types, and Great Hall
/// of the Biblioplex gives a triggered ability.
pub fn animate(g: &mut dyn Game, source: ObjectId, permanent: ObjectId, a: &Animation, duration: Duration) {
    if !g.is_on_battlefield(permanent) || g.obj(permanent).became.is_some() {
        return;
    }
    // Each part of "becomes a P/T ... creature with ..." is one instruction (`Game::one_instruction`): the state triggers
    // judge the permanent animated, not part-way.
    let _ = g.one_instruction(&mut |g| {
        g.add_effect(source, permanent, EffectKind::AddTypes(a.types.with(CardType::Creature)), duration);
        if a.replaces_creature_types {
            g.add_effect(source, permanent, EffectKind::LoseAllCreatureTypes, duration);
        }
        for &subtype in a.subtypes {
            g.add_effect(source, permanent, EffectKind::AddSubtype(subtype), duration);
        }
        if a.all_creature_types {
            g.add_effect(source, permanent, EffectKind::AddAllCreatureTypes, duration);
        }
        if let Some(colors) = a.colors {
            g.add_effect(source, permanent, EffectKind::SetColors(colors), duration);
        }
        g.add_effect(source, permanent, EffectKind::AddKeywords(a.keywords), duration);
        if let Some(ability) = a.ability {
            g.add_effect(source, permanent, EffectKind::AddAbility(ability), duration);
        }
        g.add_effect(source, permanent, EffectKind::SetPt(a.power, a.toughness), duration);
        Ok(())
    });
}

/// Whether two objects share a name (CR 201.2): an object with two names (a split or double-faced card, "A // B") shares
/// one with anything that has either, a Room on the battlefield has those of its unlocked doors alone (CR 709.5), and a
/// copy has the name it copies (`library::object_names`). Echoing Return, Echoing Ruin and Echoing Truth print "all other
/// [things] with the same name as that [thing]", and Bloodbond March and Deadly Cover-Up "with the same name".
pub fn same_name(q: &dyn Query, a: ObjectId, b: ObjectId) -> bool {
    let theirs = library::object_names(q, b);
    library::object_names(q, a).iter().any(|name| theirs.contains(name))
}

/// Explore (CR 701.44): the permanent's controller reveals the top card of their library; a land goes into their hand,
/// otherwise (a nonland card, or no card at all) the permanent gets a +1/+1 counter (if it is still the permanent it was) and they
/// may put the card into their graveyard. Fanatical Offering's Map token, Seasoned Dungeoneer and Amalia Benavides Aguirre explore it.
pub fn explore(g: &mut dyn Game, permanent: ObjectId) -> Result<(), Suspend> {
    let who = g.obj(permanent).controller;
    let top = top_of_library(g, who, 1).first().copied();
    if let Some(top) = top {
        g.reveal(who, &[top]);
        if g.chars(top).is(CardType::Land) {
            g.move_together_revealed(&[(top, Zone::hand(who))], MoveReason::Other);
            return Ok(());
        }
    }
    // "Otherwise": a nonland card, or no card at all when the library is empty (CR 701.44a-b).
    if is_live(g, permanent) && g.is_on_battlefield(permanent) {
        g.add_counters(permanent, CounterKind::PlusOne, 1);
    }
    if let Some(top) = top {
        if matches!(g.ask(who, RequestKind::May { source: top, purpose: Purpose::PutIntoGraveyard })?, Answer::Yes) {
            g.move_to(top, Zone::graveyard(who));
        }
    }
    Ok(())
}

/// "Look at the top `count` cards of your library. You may reveal a noncreature, nonland card from among them and put
/// it into your hand. Put the rest on the bottom of your library in a random order." Narset, Parter of Veils' -2 and
/// Thundertrap Trainer's enters ability. With no such card among them nobody is asked.
pub fn look_for_noncreature_nonland(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    look_for_one(g, player, source, count, |chars| !chars.is(CardType::Creature) && !chars.is(CardType::Land))
}

/// "Look at the top `count` cards of your library. You may reveal a creature or land card from among them and put it
/// into your hand. Put the rest on the bottom of your library in a random order." Once Upon a Time. With no such card
/// among them nobody is asked.
pub fn look_for_creature_or_land(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    look_for_one(g, player, source, count, |chars| chars.is(CardType::Creature) || chars.is(CardType::Land))
}

/// The look both of the above make: one card `wanted` may go to the hand, the rest to the bottom at random.
fn look_for_one(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize, wanted: fn(&Chars) -> bool) -> Result<(), Suspend> {
    library::look_at_top(g, player, source, count, (0, 1), true, wanted, library::Rest::BottomRandom)?;
    Ok(())
}

/// "Each player shuffles their hand and graveyard into their library, then draws seven cards": every hand and
/// graveyard moves together (CR 701.24), the libraries are shuffled, and each player draws in turn order from the
/// active player (CR 101.4); nobody is asked. Echo of Eons, Timetwister, Time Spiral and Day's Undoing print it.
pub fn shuffle_hands_and_graveyards_in_and_draw_seven(g: &mut dyn Game) {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let mut cards: Vec<ObjectId> = Vec::new();
    for &p in &players {
        cards.extend(g.state().zone(Zone::hand(p)).iter().chain(g.state().zone(Zone::graveyard(p)).iter()).copied());
    }
    g.put_into_library(&cards, LibraryPlace::Top);
    for &p in &players {
        g.shuffle(p);
    }
    for &p in &players {
        g.draw(p, 7);
    }
}

/// Hideaway N (CR 702.75a): look at the top N cards of the library, exile one of them (exactly one, with fewer than N
/// cards it is one of those there are), and put the rest on the bottom in a random order. The exiled card is linked to
/// the resolving ability (CR 607.2a), so its own "play the exiled card" ability finds it with `Query::exiled_with`. Windbrisk Heights,
/// Shelldock Isle and Collector's Cage print it.
pub fn hideaway(g: &mut dyn Game, item: &StackItem, count: usize) -> Result<(), Suspend> {
    let (player, source) = (item.controller, item.object);
    let looked = top_of_library(g, player, count);
    if looked.is_empty() {
        return Ok(());
    }
    let ask = RequestKind::DistributeCards {
        source,
        cards: looked.clone(),
        destinations: vec![
            Destination { kind: DestinationKind::Exile, min: 1, max: 1, eligible: looked.clone() },
            Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u8::MAX, eligible: looked.clone() },
        ],
        may_shuffle: false,
    };
    let mut chosen = None;
    if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
        chosen = placements.first().and_then(|p| p.first().copied());
    }
    // Everything not exiled goes to the bottom in a random order, whether or not the exile itself succeeds.
    let rest: Vec<ObjectId> = looked.into_iter().filter(|&c| Some(c) != chosen).collect();
    if let Some(card) = chosen {
        // Exiled face down (CR 406.3); the player who looked at it may go on looking.
        let exiled = g.exile_face_down(&[card], Some(g.linked(item)));
        g.look_at(player, &exiled);
    }
    g.bottom_in_random_order(player, &rest);
    Ok(())
}
