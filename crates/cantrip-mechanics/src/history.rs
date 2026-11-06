//! The questions about the past that several printed cards ask in the same words, written once over the event
//! history (`cantrip_api::history`). A trigger matching an event and a replacement deciding about one both see the
//! history as it was just before it, so each of these reads the same from either.

use cantrip_api::*;

/// "This ability has been activated N times this turn" (CR 113.2c, 602.5c), for the exact selected instance.
/// Vampire Bats and Farrelite Priest count their own independently acquired copies this way.
pub fn this_ability_activations_this_turn(q: &dyn Query, asker: &Asker) -> usize {
    let instance = asker.instance.expect("an activated ability is asked with its exact instance");
    q.history().select(EventQuery::kind(EventMask::ABILITY_ACTIVATED).object(asker.source).this_turn())
        .filter(|r| matches!(r.event, Event::AbilityActivated { ability, instance: key, .. }
            if ability == instance.ability && key == instance.acquisition))
        .count()
}

/// The `Cast` record of the spell `id`, or of the spell the permanent `id` became (CR 400.7, 608.3): the latest one. A copy of a
/// spell was not cast (CR 707.10) and has none, and a permanent put onto the battlefield was never a spell.
pub fn cast_of(q: &dyn Query, id: ObjectId) -> Option<Cast> {
    q.history().all().iter().rev().find_map(|r| match &r.event {
        Event::Cast(c) if c.spell == id || q.obj(c.spell).became == Some(id) => Some(*c),
        _ => None,
    })
}

/// What kind of way a cast used (`WayKind`): the way a permanent gave it (`GrantedWay::kind`), or else the card's own way it was
/// cast by; `None` for the card's mana cost (no way) or a way no keyword's. Escape (CR 702.138b), mayhem (CR 702.187) and sneak
/// (CR 702.190) ask it. A way a permanent gave casts the face for its cost; the card's own ways are not open under another
/// permission.
pub fn way_kind_of(q: &dyn Query, cast: Cast) -> Option<WayKind> {
    match cast.granted_by() {
        Some(grant) => grant.ability.as_granted_way().and_then(|k| q.card(grant.ability.card).granted_ways.get(k)).map(|w| w.kind),
        None => (cast.way > 0).then(|| q.card(q.obj(cast.spell).card).ways.get(cast.way as usize - 1)).flatten().map(|w| w.kind),
    }
}

/// The kind of way the spell `id` (or the spell the permanent `id` became) was cast by: `way_kind_of` its `cast_of`. Escape's
/// "escaped" and mayhem's "mayhem cost was paid" are `Some(WayKind::Escape)` and `Some(WayKind::Mayhem)` of it.
pub fn cast_way_kind(q: &dyn Query, id: ObjectId) -> Option<WayKind> {
    way_kind_of(q, cast_of(q, id)?)
}

/// "A creature that dealt damage this turn" (CR 120.2): `source` dealt damage this turn, to anything, combat or not. Damage that
/// was prevented, or whose amount was zero, was not dealt (CR 615.6). The object is the one that dealt it, so a permanent that left
/// and came back is another (CR 400.7). Red Guardian, Super-Soldier, Avenging Arrow and Executioner's Swing print it.
pub fn dealt_damage_this_turn(history: History<'_>, source: ObjectId) -> bool {
    history.damage_dealt().any(|d| d.source == source && d.amount > 0)
}

/// "[It] was dealt damage this turn" (CR 120.2): `object` was the recipient of damage this turn, combat or not. Damage that was
/// prevented, or whose amount was zero, was not dealt (CR 615.6). The object is the one that was dealt it, so a permanent that
/// left and came back has been dealt none (CR 400.7). Faller's Faithful and Sold Out print it.
pub fn was_dealt_damage_this_turn(history: History<'_>, object: ObjectId) -> bool {
    history.damage_dealt().any(|d| d.target == Target::Object(object) && d.amount > 0)
}

/// "You've put one or more +1/+1 counters on [it] this turn" (CR 122.6, 611.3a): `player` put at least one +1/+1 counter on
/// `object` this turn, whether it was on the battlefield or entering with them (the counters' putter is the event's). The object
/// is the one that received them, so a permanent that left and came back has had none put on it (CR 400.7). Kid Loki and Beast,
/// Erudite Aerialist ask it from their static abilities' layers.
pub fn put_plus_one_counter_on_this_turn(history: History<'_>, player: PlayerId, object: ObjectId) -> bool {
    history
        .select(EventQuery::kind(EventMask::COUNTERS).this_turn().object(object))
        .any(|record| matches!(record.event, Event::CountersPut { kind: CounterKind::PlusOne, amount, by, .. } if amount > 0 && by == player))
}

/// How the spell on the stack as `spell` was cast, if it was cast from a graveyard (CR 601.2i, 400.7): the cast the history
/// holds, kept only when its `from` is a graveyard (the graveyard of the spell's owner, CR 404.1). A copy was not cast, and a
/// permanent put onto the battlefield was never a spell, so neither has one. A trigger matching the entry of a permanent
/// asks it of the spell that resolved (`ZoneChange::old`), the history as it was just before the entry. Twilight Diviner and
/// Prized Amalgam print "cast from a graveyard".
pub fn cast_from_graveyard<'a>(history: History<'a>, spell: ObjectId) -> Option<&'a Cast> {
    history.cast_of(spell).filter(|cast| cast.from == ZoneKind::Graveyard)
}

/// The cast from a graveyard of the spell `permanent` entered the battlefield as (CR 608.3, 400.7), once the entry is in the
/// history: the spell is the object of the resolution that put `permanent` onto the battlefield (`cast_from_graveyard`).
/// Nothing for a permanent that was not a spell as it entered. Prized Amalgam asks it as its delayed trigger's condition
/// is checked as it resolves (CR 603.4).
pub fn cast_from_graveyard_as_it_entered<'a>(history: History<'a>, permanent: ObjectId) -> Option<&'a Cast> {
    history
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).reason(ZoneReason::Resolve))
        .rev()
        .find_map(|r| match r.event {
            Event::ZoneChange { old, new, .. } if new == permanent => Some(old),
            _ => None,
        })
        .and_then(|spell| cast_from_graveyard(history, spell))
}

/// An opponent was dealt noncombat damage this turn (CR 120.2). Grim Repriser and Whiplash Wordsmith ask this
/// same question, from an activated ability's condition and a static ability's layers respectively.
pub fn opponent_dealt_noncombat_damage_this_turn(history: History<'_>, state: &GameState, player: PlayerId) -> bool {
    history.this_turn().iter().any(|r| {
        matches!(r.event, Event::Damage { target: Target::Player(p), amount, kind, .. }
            if !kind.is_combat() && amount > 0 && state.opponents(player).any(|opponent| opponent == p))
    })
}

/// Whether a draw `player` is about to make (or is making, from a trigger on it) is the first card they draw in their
/// own draw step this turn: the draw that "except the first one they draw in each of their draw steps" exempts. Orcish
/// Bowmasters (a trigger) and Hullbreacher (a replacement) print it.
pub fn first_draw_in_own_draw_step(q: &dyn Query, player: PlayerId) -> bool {
    let turn = q.state().turn;
    turn.active == player
        && turn.step() == Step::Draw
        && !q.history().any(EventQuery::kind(EventMask::DREW).player(player).this_turn().step(Step::Draw))
}

/// Cards `player` has discarded this turn (CR 701.9), as the game is now: "for each card you've discarded this turn", a discard
/// made as a cost or earlier in the same resolution included. Jiang Yanggu, Alone and Misty Knight, Hero for Hire print it.
pub fn cards_discarded_this_turn(q: &dyn Query, player: PlayerId) -> u32 {
    q.history().count(EventQuery::kind(EventMask::DISCARDED).player(player).this_turn()) as u32
}

/// Cards `player` has drawn this turn, before the draw being asked about (Faerie Mastermind's "second card", Leovold's
/// and Narset's "more than one card each turn").
pub fn drawn_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history().drawn(player)
}

/// "Draws their second card each turn" (CR 120.3, 603.2): the player making this draw, if it is their second card this
/// turn, whichever step it falls in (the history holds the draws before the one being matched). Faerie Mastermind ("an
/// opponent") and King T'Challa ("a player") print it.
pub fn second_card_drawn_this_turn(q: &dyn Query, event: &Event) -> Option<PlayerId> {
    match event {
        Event::Drew { player, .. } if drawn_this_turn(q, *player) == 1 => Some(*player),
        _ => None,
    }
}

/// Spells cast this turn by every player (storm, CR 702.40a, counts those cast before the storm spell: subtract it).
pub fn spells_cast_this_turn(q: &dyn Query) -> usize {
    q.history().casts().count()
}

/// Revolt (CR 207.2c): whether a permanent `player` controlled left the battlefield this turn. The leaving object's
/// row is as it was when it left, so its controller then is read from it.
pub fn permanent_left_battlefield_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { old, .. } if q.obj(old).controller == player))
}

/// Disappear (CR 207.2c): "if a permanent left the battlefield under your control this turn", the same question revolt asks,
/// so the same answer, for the ability's controller. It is the `condition` of the triggered ability (CR 603.4: asked as it
/// would trigger and again as it resolves). Foot Mystic, Insectoid Exterminator, Lord Dregg, Insect Invader and Rat King print it.
pub fn disappear(q: &dyn Query, asker: &Asker) -> bool {
    permanent_left_battlefield_this_turn(q, asker.controller)
}

/// "Another creature entered the battlefield under your control this turn" (CR 603.6a): a creature other than `except` entered
/// under `player`'s control this turn, judged as it entered (the controller and card types `Event::Entered` holds), so one that has
/// since died, been stolen or stopped being a creature still counts and one that entered under an opponent's control and was
/// stolen does not. `except` is the object itself; a permanent that left and came back is another (CR 400.7). Bristlebane Outrider
/// asks it from its static ability's layers and Wary Farmer from a triggered ability's condition.
pub fn another_creature_entered_under_control_this_turn(history: History<'_>, player: PlayerId, except: ObjectId) -> bool {
    history.entered().any(|e| e.permanent != except && e.controller == player && e.is(CardType::Creature))
}

/// The number of creatures that entered the battlefield under `player`'s control this turn (CR 603.6a), judged as each entered (the
/// controller and card types `Event::Entered` holds), tokens included, whatever has happened to it since. Kinbinding sums them into a
/// bonus; Spider-UK asks whether there were two or more.
pub fn creatures_entered_under_control_this_turn(history: History<'_>, player: PlayerId) -> usize {
    history.entered().filter(|e| e.controller == player && e.is(CardType::Creature)).count()
}

/// "An artifact entered the battlefield under your control this turn" (CR 603.6a): a permanent that entered this turn under
/// `player`'s control and was an artifact as it entered (the controller and card types `Event::Entered` holds), whatever it is
/// or who controls it now - one that has since died, been stolen or stopped being an artifact still counts. Iron Man, Fugitive
/// Droid and Mechan Shieldmate ask it.
pub fn artifact_entered_under_control_this_turn(history: History<'_>, player: PlayerId) -> bool {
    history.entered().any(|e| e.controller == player && e.is(CardType::Artifact))
}

/// The number of permanents `player` has sacrificed this turn (CR 701.21a): every permanent that went from the battlefield to a
/// graveyard as a sacrifice this turn, token or not, under that player's control as it left. Asked of the history (CR 603.10a).
/// Obsessive Pursuit counts them; Phoenix Fleet Airship asks whether there was one ("if you sacrificed a permanent this turn").
pub fn permanents_sacrificed_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .filter(|r| crate::triggers::sacrificed_by(q, player, &r.event).is_some())
        .count()
}

/// The void condition: "a nonland permanent left the battlefield this turn or a spell was warped this turn". Void is an
/// ability word (CR 207.2c) with no rules of its own; only its second clause has a rule, CR 702.185c: a spell was warped when
/// it was cast for its warp cost. Any player's permanent counts, a token too, judged as it was as it left the battlefield (CR 608.2h). Elegy Acolyte, Plasma Bolt and
/// every other card with a void ability print it.
pub fn void(q: &dyn Query) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { old, .. } if q.state().last_known(old).is_some_and(|l| !l.types.has(CardType::Land))))
        || crate::warp::spell_was_warped_this_turn(q)
}

/// Cards (not tokens) that left `player`'s graveyard this turn (Gau, Feral Youth; Wilt in the Heat).
pub fn cards_left_graveyard_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Graveyard).this_turn())
        .filter(|r| matches!(r.event, Event::ZoneChange { old, from, .. } if from == Zone::graveyard(player) && !q.obj(old).token))
        .count()
}

/// The cards put into `owner`'s graveyard from the battlefield this turn that are still there (Faith's Reward, Second
/// Sunrise, No Rest for the Wicked): the graveyard objects those moves made, in the order they arrived.
pub fn put_into_graveyard_from_battlefield_this_turn(q: &dyn Query, owner: PlayerId) -> Vec<ObjectId> {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Graveyard).this_turn())
        .filter_map(|r| match r.event {
            // A token that left the battlefield is not a card in the graveyard (CR 111.7, 704.5d) - the token itself, as it
            // arrived: a merged permanent's card components are cards though its topmost is a token, and each came from the
            // battlefield with it (CR 730.3, `Event::moved_from`).
            Event::ZoneChange { new, to, .. }
                if r.event.moved_from() == Some(Zone::BATTLEFIELD) && to == Zone::graveyard(owner) && q.obj(new).became.is_none() && !q.obj(new).token =>
            {
                Some(new)
            }
            _ => None,
        })
        .collect()
}

/// Whether `permanent` entered the battlefield this turn (CR 400.7: it is the object that move made). Asked of a history
/// so that a static ability's condition reads it too (`LayerQuery::history`): Shardmage's Rescue's "as long as this Aura
/// entered this turn", The Wandering Emperor's, power-up's cost.
pub fn entered_this_turn(history: History<'_>, permanent: ObjectId) -> bool {
    history.entered_this_turn(permanent)
}

/// How many of `player`'s opponents have lost life this turn (spectacle's "an opponent lost life this turn", Drill
/// Bit; Kaito, Bane of Nightmares counts them).
pub fn opponents_who_lost_life_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    let h = q.history();
    q.state().opponents(player).filter(|&p| h.life_lost(p) > 0).count()
}

/// Whether an opponent of `player` has lost life this turn (CR 119.3: "lost life", not a lower life total - one who gained 2
/// and lost 1 still did). Spectacle, Fireglass Mentor and Flamecache Gecko ask it.
pub fn an_opponent_lost_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    opponents_who_lost_life_this_turn(q, player) > 0
}

/// `an_opponent_lost_life_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if an
/// opponent lost life this turn" (CR 603.4) - Fireglass Mentor, Flamecache Gecko - and spectacle's.
pub fn an_opponent_lost_life(q: &dyn Query, asker: &Asker) -> bool {
    an_opponent_lost_life_this_turn(q, asker.controller)
}

/// Whether `object` arrived where it is this turn (CR 400.7: it is the object a move this turn made), in whatever zone:
/// Night Nurse's "put there from anywhere this turn", and, on the battlefield, `entered_this_turn`.
pub fn arrived_this_turn(q: &dyn Query, object: ObjectId) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if new == object))
}

/// The creatures that died this turn (CR 700.4): every move from the battlefield to a graveyard this turn of an object
/// that was a creature as it left, by last known information, so a token that has since ceased to exist still counts.
/// With `under`, only those that player controlled as they died (Callous Sell-Sword's "under your control"; the leaving
/// object's row is as it was when it left). Spymaster's Vault counts them all.
pub fn creatures_died_this_turn(q: &dyn Query, under: Option<PlayerId>) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).to(ZoneKind::Graveyard).this_turn())
        .filter(|r| {
            matches!(r.event, Event::ZoneChange { old, .. }
                if q.state().last_known(old).is_some_and(|l| l.types.has(CardType::Creature))
                    && under.is_none_or(|p| q.obj(old).controller == p))
        })
        .count()
}

/// "If you gained life this turn" (CR 603.4, 119.3): whether `player` has gained any life this turn. Ocelot Pride's and
/// Enlightened Confidant's intervening ifs print it.
pub fn gained_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().life_gained(player) > 0
}

/// `gained_life_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if you gained life this
/// turn" (CR 603.4). Lunar Convocation and Moseo, Vein's New Dean print it.
pub fn you_gained_life_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    gained_life_this_turn(q, asker.controller)
}

/// "If you've gained 5 or more life this turn" (CR 603.4, 119.3): whether the ability's controller has gained at least five life
/// this turn, as an intervening "if" or a "to solve" condition. Resplendent Angel and Case of the Uneaten Feast print it.
pub fn you_gained_five_or_more_life_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().life_gained(asker.controller) >= 5
}

/// "If you lost life this turn" (CR 603.4, 119.3): whether `player` has lost any life this turn, however much they also gained
/// (damage and paying life count). Lunar Convocation and Thought-Stalker Warlock print it.
pub fn lost_life_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().life_lost(player) > 0
}

/// Whether `player` has activated an equip ability this turn (CR 702.6a, 602.2): what "the first equip ability you activate
/// [during each of your turns / each turn]" asks of an equip ability being announced, which is not yet in the history.
/// Forge Anew and Kíli the Resourceful print it.
pub fn activated_an_equip_ability_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::AbilityActivated { ability, player: p, .. }
            if p == player && q.activated_ability(ability).is_some_and(|d| d.equip))
    })
}

/// "Choose one that hasn't been chosen" (CR 700.2): whether mode `mode` of a modal ability has not been chosen before by the
/// object that has it (`asker.source`), as its ability went on the stack. A mode chosen for an ability that was then countered
/// or fizzled stays chosen, whoever controlled the object when it was chosen, and an object that left and came back is a new one
/// (CR 400.7) with every mode to choose again. Gollum, Riddle Master and Demonic Pact print it (`ModalDef::allowed`).
pub fn mode_not_chosen_before(q: &dyn Query, asker: &Asker, mode: u8) -> bool {
    !q.history().all().iter().any(|r| {
        matches!(r.event, Event::ModesChosen { source, modes, .. } if source == asker.source && modes & (1 << mode) != 0)
    })
}

/// "Choose one that hasn't been chosen this turn" (CR 700.2): whether mode `mode` has not been chosen this turn by the object
/// that has the ability (`asker.source`), as its ability went on the stack. A mode chosen for an ability that was then
/// countered stays chosen, and an object that left and came back is a new one (CR 400.7) with its own. With every mode chosen
/// the ability still triggers and is removed as it would be put on the stack (CR 603.3c). Monument to Endurance and The Vision
/// print it (`ModalDef::allowed`).
pub fn mode_not_chosen_this_turn(q: &dyn Query, asker: &Asker, mode: u8) -> bool {
    !q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::ModesChosen { source, modes, .. } if source == asker.source && modes & (1 << mode) != 0)
    })
}

/// "You face a dilemma as you choose one or more modes for a spell or ability" (CR 700.2a, 700.2b): the player who faced
/// one in `event`, if it is one - modes chosen as a spell is cast (CR 601.2b), an ability activated (CR 602.2b) or a
/// triggered ability put on the stack (CR 603.3c). However many modes are chosen, the same one more than once too (CR
/// 700.2d), it is one dilemma; a "choose up to" spell or ability that chose none faced none, and a copy, which copies its
/// modes, chooses none (CR 700.2g). What "whenever you face a dilemma" (`triggers::YOU_FACE_A_DILEMMA`) triggers on and
/// what Captain Benjamin Sisko counts this turn.
pub fn faced_a_dilemma(event: &Event) -> Option<PlayerId> {
    match *event {
        Event::ModesChosen { player, modes, .. } if modes != 0 => Some(player),
        _ => None,
    }
}

/// "The sacrificed creature's power" (CR 601.2h, 608.2h, 707.10): the creature paid for the spell's own first
/// printed additional cost, as it last existed. A copy uses the original spell's cost object. Fling and Kazuul's Fury print it.
pub fn sacrificed_creatures_power(q: &dyn Query, spell: ObjectId) -> i32 {
    q.history().spell_cost_objects(spell, CostOrigin::PrintedAdditional, 0, CostPart::Permanents)
        .next().map_or(0, |creature| q.chars(creature).power.unwrap_or(0) as i32)
}

/// "That creature's power" of a creature a resolving ability sacrificed (CR 701.21, 608.2h): the power of the permanent that
/// `sacrificed` (the new object in the graveyard, as `effects::may_sacrifice` returns it) was before the sacrifice, as it last
/// existed on the battlefield, pumps, counters and copy effects included. It is the old object of the zone change that made
/// `sacrificed` (CR 400.7); zero for an object no zone change made. Bolg of the North and Rhovanion Rampager print it.
pub fn power_of_sacrificed_permanent(q: &dyn Query, sacrificed: ObjectId) -> i32 {
    sacrificed_permanent(q, sacrificed).map_or(0, |old| crate::targets::power_or_last_known(q, old))
}

/// What a sacrificed permanent was on the battlefield (CR 701.21, 400.7): the old object of the zone change that made `sacrificed`
/// (the new object in the graveyard, as `effects::may_sacrifice` returns it), whose characteristics are last-known information
/// (CR 608.2h); `None` for an object no zone change made.
fn sacrificed_permanent(q: &dyn Query, sacrificed: ObjectId) -> Option<ObjectId> {
    q.history().all().iter().rev().find_map(|r| if let Event::ZoneChange { old, new, .. } = r.event && new == sacrificed { Some(old) } else { None })
}

/// "The sacrificed permanent's mana value" of a permanent a resolving ability sacrificed (CR 701.21, 608.2h, 202.3): the mana
/// value of the permanent that `sacrificed` (the new object in the graveyard, as `effects::may_sacrifice` returns it) was before
/// the sacrifice, as it last existed on the battlefield, copy effects included. It is the old object of the zone change that
/// made `sacrificed` (CR 400.7); zero for an object no zone change made. Enigmatic Incarnation and Eddie Brock print it.
pub fn mana_value_of_sacrificed_permanent(q: &dyn Query, sacrificed: ObjectId) -> u32 {
    sacrificed_permanent(q, sacrificed).map_or(0, |old| q.mana_value(old))
}

/// "Each creature that convoked it" (CR 702.51c, 109.2): the creatures that convoked `object` (a spell, or the permanent it
/// became) that are creatures on the battlefield now. A convoker that has since left the battlefield is a new object
/// wherever it went (CR 400.7), and one that is no longer a creature is not a creature that convoked it (CR 109.2: March of
/// the Machines ended), so nothing happens to either. Venerated Loxodon and Zephyr Singer print it.
pub fn convoking_creatures_on_the_battlefield(q: &dyn Query, object: ObjectId) -> Vec<ObjectId> {
    q.history()
        .convoked(object)
        .filter(|&creature| is_live(q, creature) && q.obj(creature).zone.is(ZoneKind::Battlefield) && q.chars(creature).is(CardType::Creature))
        .collect()
}

/// "Your first, second, or third turn of the game" (CR 500, 102.1): it is `player`'s own turn and they have begun at most `n`
/// turns, an extra turn counting as one. Starting Town's enters-tapped clause and Jace Reawakened's casting restriction print it
/// (with `n` = 3).
pub fn in_your_first_turns(q: &dyn Query, player: PlayerId, n: u32) -> bool {
    q.state().turn.active == player
        && q.history().select(EventQuery::kind(EventMask::TURN_BEGAN).player(player)).nth(n as usize).is_none()
}

/// "From the Future - You can't cast this spell during your first, second, or third turns of the game" (CR 601.3, 500): a card's
/// `cast_only_if`, true while it is not one of the caster's own first three turns. Jace Reawakened and Spider-Man 2099 print it.
pub fn from_the_future(q: &dyn Query, asker: &Asker) -> bool {
    !in_your_first_turns(q, asker.controller, 3)
}

/// "If you've completed a dungeon" (CR 309.7): whether `player` has completed one this game. Caves of Chaos Adventurer,
/// Undermountain Adventurer and White Plume Adventurer print it.
pub fn completed_a_dungeon(q: &dyn Query, player: PlayerId) -> bool {
    q.history().any(EventQuery::kind(EventMask::DUNGEON_COMPLETED).player(player))
}

/// The first noncreature spell a player casts this turn (CR 603.2), including spells cast before
/// a permanent entered. Plan for All Outcomes and The Queen of Dale ask this same history question.
pub fn first_noncreature_spell_this_turn(q: &dyn Query, spell: ObjectId, player: PlayerId) -> bool {
    !q.history().casts_by(player).any(|c| c.spell != spell && !c.is(CardType::Creature))
}

/// "If you've scried or surveilled this turn" (CR 701.22, 701.25): `player` completed a scry or a surveil this turn, of any
/// number of cards. Desperate Futurescribe and Proctor of Potential ask it.
pub fn scried_or_surveilled_this_turn(q: &dyn Query, player: PlayerId) -> bool {
    q.history()
        .this_turn()
        .iter()
        .any(|r| matches!(r.event, Event::Scried { player: p, .. } | Event::Surveilled { player: p, .. } if p == player))
}

/// Whether the permission or the alternative cost `source` gives has been used this turn: a spell cast or a land played by
/// it, or a spell cast for it, as the history records what each was made by and paid with (`Cast::granted_by`,
/// `Event::LandPlayed`, `Event::PaidAlternative`). "Once during each of your turns, you may cast ... from your graveyard"
/// (CR 601.3): Lurrus of the Dream-Den, and Serra Paragon, whose land plays share the once; "once during each of your
/// turns, you may cast ... from your hand without paying its mana cost" (CR 118.9): Zaffai and the Tempests, Vision,
/// Spectral Synthezoid. It is that object's own: another permanent with the same ability is another permission, and one
/// that left and came back is a new object (CR 400.7).
pub fn permission_used_this_turn(q: &dyn Query, source: ObjectId) -> bool {
    q.history().this_turn().iter().any(|r| match r.event {
        Event::Cast(c) => c.granted_by().is_some_and(|g| g.object == source),
        Event::LandPlayed { granted_by, .. } => granted_by.some().is_some_and(|g| g.object == source),
        Event::PaidAlternative { by, .. } => by.object == source,
        _ => false,
    })
}

/// The permanents `source` dealt damage to (CR 120.3) in the records from index `since` on (`history().head()` noted before
/// the damage), each once, in the order dealt: read from the damage events, so damage that was prevented names none and a
/// redirection (CR 614.9, Ancient Adamantoise) names the permanent the damage was dealt to, not the one aimed at. Anger of
/// the Gods, Redcap Melee, The Black Arrow and Torch the Tower print "dealt damage this way" about them.
pub fn permanents_dealt_damage_by(history: History, source: ObjectId, since: usize) -> Vec<ObjectId> {
    let mut dealt: Vec<ObjectId> = Vec::new();
    for r in &history.all()[since..] {
        if let Event::Damage { source: s, target: Target::Object(id), amount, .. } = r.event {
            if s == source && amount > 0 && !dealt.contains(&id) {
                dealt.push(id);
            }
        }
    }
    dealt
}

/// The players `source` dealt damage to (CR 120.3) in the records from index `since` on (`history().head()` noted before the
/// damage), each once, in the order dealt: read from the damage events, so damage that was prevented names none and a
/// redirection (CR 614.9) names the player the damage was dealt to, not the one aimed at. Screaming Nemesis and Sonic Shrieker
/// print "if a player is dealt damage this way" about them.
pub fn players_dealt_damage_by(history: History, source: ObjectId, since: usize) -> Vec<PlayerId> {
    let mut dealt: Vec<PlayerId> = Vec::new();
    for r in &history.all()[since..] {
        if let Event::Damage { source: s, target: Target::Player(p), amount, .. } = r.event {
            if s == source && amount > 0 && !dealt.contains(&p) {
                dealt.push(p);
            }
        }
    }
    dealt
}

/// "If you cast this spell during your main phase" (CR 505.1, 601.2i): a `CardDef::cast_fact`, 1 if the spell's caster was
/// the active player in a precombat or postcombat main phase as it became cast, else 0. Read with
/// `history().fact_as_cast`, so a copy (not cast, CR 707.10) never has it. Tail Swipe, Might of Old Krosa, Haunting Hymn and
/// the other "Addendum" spells print it.
pub fn cast_during_your_main_phase(q: &dyn Query, me: &Asker) -> u32 {
    let turn = q.state().turn;
    (turn.active == me.controller && turn.step().is_main()) as u32
}

/// "Attacked with N creatures this turn" (CR 508.1): how many different creatures were declared as attackers this turn by
/// `player`, who declares them only on their own turn (CR 506.2, 508.1a), so any other player has attacked with none - an
/// ability's controller need not be the active player when a copy of it is controlled by another (CR 707.10). A creature
/// that attacked in two combats counts once, one put onto the battlefield attacking was never declared, and the creature
/// need not still be on the battlefield or under anyone's control. Windbrisk Heights, Wedding Announcement and Chart a
/// Course (whether it is any) ask it.
pub fn creatures_attacked_with(q: &dyn Query, player: PlayerId) -> usize {
    let mut seen: Vec<ObjectId> = Vec::new();
    for a in q.history().attacked_with(player) {
        if !seen.contains(&a.attacker) {
            seen.push(a.attacker);
        }
    }
    seen.len()
}

/// "If you didn't attack with a creature this turn" (CR 508.1, 603.4) as an intervening "if" (`AbilityDef::condition`): no
/// creature was declared as an attacker this turn by the ability's controller (`creatures_attacked_with`). Curious
/// Obsession and See Red print it.
pub fn you_didnt_attack_with_a_creature(q: &dyn Query, me: &Asker) -> bool {
    creatures_attacked_with(q, me.controller) == 0
}

/// This combat's records (CR 506.1): those of this turn since its latest beginning of combat step began, so a second combat
/// phase in a turn (CR 500.8) has its own; empty before the turn's first combat. Melee's "each opponent you attacked this
/// combat" and Kytheon, Hero of Akros's "if Kytheon and at least two other creatures attacked this combat" ask it.
pub fn this_combat(q: &dyn Query) -> &[Record] {
    let turn = q.history().this_turn();
    match turn.iter().rposition(|r| matches!(r.event, Event::StepBegan { step: Step::BeginCombat })) {
        Some(began) => &turn[began..],
        None => &[],
    }
}

/// "If it's the first combat phase of the turn" (CR 603.4, 500.8) as an intervening "if" (`AbilityDef::condition`): the turn's
/// beginning of combat steps are in its history, and the first combat phase is the only one of them so far. It is asked as the
/// ability triggers and again as it resolves. Raph & Leo, Sibling Rivals and Genji Glove print it.
pub fn first_combat_phase_of_the_turn(q: &dyn Query, _: &Asker) -> bool {
    q.history().this_turn().iter().filter(|r| matches!(r.event, Event::StepBegan { step: Step::BeginCombat })).count() == 1
}

/// `cards_left_graveyard_this_turn` as a condition of the ability's controller: a triggered ability's intervening "if a card
/// left your graveyard this turn" (CR 603.4). Gau, Feral Youth and Primary Research print it.
pub fn a_card_left_your_graveyard_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    cards_left_graveyard_this_turn(q, asker.controller) > 0
}
