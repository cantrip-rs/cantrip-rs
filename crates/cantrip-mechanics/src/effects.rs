//! One-line effects as functions a `TriggeredDef` or `ActivatedDef` can name, parameterised the way `ward::<N>` is, so
//! a card does not write a wrapper around a single `Game` call. Each acts for the ability's controller or its source.

use cantrip_api::*;

/// "This creature enters with X +1/+1 counters on it" (CR 614.1c), for the X it was cast with; put onto the battlefield
/// without being cast, X is 0. Walking Ballista, Guiding Hydra and the ravenous creatures print it.
pub fn enters_with_x_counters(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += entering.x as i16;
    Ok(())
}

/// "This creature enters with `N` -1/-1 counters on it" (CR 614.1c). Glen Elendra Guardian, Flitterwing Nuisance (one)
/// and Moonshadow (six) print it.
pub fn enters_with_minus_one_counters<const N: i16>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::MinusOne as usize] += N;
    Ok(())
}

/// "Target creature gets +`P`/+`T` until end of turn" (CR 611.2a), the creature being the ability's first target; nothing
/// if it is no longer a legal target as the spell or ability resolves (CR 608.2b). Bedeck // Bedazzle, Collective
/// Brutality, Dismember, Fervent Champion, Giant Growth, Grim Bauble, Harvester of Misery, Icy Reception, Last Gasp,
/// Locust Spray, Proft, Stab, The Mightstone and Weakstone, Theorix Charm, Umezawa's Jitte and Shambling Ghast print it.
pub fn target_creature_gets<const P: i16, const T: i16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        crate::pump(g, item.object, creature, P, T);
    }
    Ok(())
}

/// "Destroy all nonland permanents with mana value `limit` or less" (CR 701.8a): every such permanent on the battlefield,
/// destroyed at once. The permanents it destroyed, for what the rest of the text does with them. Culling Ritual (2) and
/// Pest Control (1) print it.
pub fn destroy_nonland_permanents_with_mana_value_at_most(g: &mut dyn Game, limit: u32) -> Vec<ObjectId> {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| !g.chars(id).is(CardType::Land) && g.mana_value(id) <= limit)
        .collect();
    g.destroy_all(&doomed)
}

/// "Destroy all artifacts with mana value `limit` or less" (CR 701.8a): every such artifact on the battlefield, destroyed at
/// once. The artifacts it destroyed. Brotherhood's End (3), Hammer Mage (X) and Meltdown (X) print it.
pub fn destroy_artifacts_with_mana_value_at_most(g: &mut dyn Game, limit: u32) -> Vec<ObjectId> {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Artifact) && g.mana_value(id) <= limit)
        .collect();
    g.destroy_all(&doomed)
}

/// "Creatures you control get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): the creatures `item`'s controller
/// controls as it resolves, and no creature that enters later. The creatures, for what the rest of the sentence does to them
/// ("and gain haste", "untap those creatures"). Assault Formation, Eagles of the North, Fatehold Charm, Haze of Rage, Jeskai
/// Ascendancy, Lorehold Charm, Silk Web Weaver and Warhost's Frenzy print it.
pub fn pump_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i16, toughness: i16) -> Vec<ObjectId> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &id in &creatures {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
    creatures
}

/// "Other creatures you control get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): the creatures `item`'s
/// controller controls as it resolves but its source, and no creature that enters later. The creatures, for what the rest
/// of the sentence does to them ("and gain trample"). Restless Prairie and Summon: Esper Maduin print it.
pub fn pump_other_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i16, toughness: i16) -> Vec<ObjectId> {
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &id in &others {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
    others
}

/// "Creatures you control gain haste until end of turn" (CR 611.2, 611.2c, 702.10): the creatures `item`'s controller controls as
/// it resolves, and no creature that enters later. Bitter Reunion and Ingris Stingerquill print it.
pub fn creatures_you_control_gain_haste(g: &mut dyn Game, item: &StackItem) {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for id in creatures {
            crate::haste_until_end_of_turn(g, item.object, id);
        }
    });
}

/// "All creatures get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): every creature on the battlefield as
/// `source`'s spell or ability resolves, whoever controls it, and no creature that enters later. Toxic Deluge (-X/-X) and
/// Withering Curse (-2/-2) print it.
pub fn pump_all_creatures(g: &mut dyn Game, source: ObjectId, power: i16, toughness: i16) {
    let creatures = creatures_on_the_battlefield(g);
    instruction(g, |g| {
        for id in creatures {
            crate::pump(g, source, id, power, toughness);
        }
    });
}

/// "Creatures you control gain trample and get +P/+T until end of turn" (CR 611.2c): `pump_creatures_you_control` and trample for
/// each of them, the creatures being those on the battlefield as it resolves. The creatures, for what the rest of the sentence does
/// to them. Craterclaw Colossus (+X/+0, X the artifacts you control), Craterhoof Behemoth (+X/+X, X the creatures) and Balmor,
/// Battlemage Captain (+1/+0) print it.
pub fn pump_and_trample_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i16, toughness: i16) -> Vec<ObjectId> {
    let mut creatures = Vec::new();
    instruction(g, |g| {
        creatures = pump_creatures_you_control(g, item, power, toughness);
        for &creature in &creatures {
            crate::trample_until_end_of_turn(g, item.object, creature);
        }
    });
    creatures
}

/// "Put `n` `kind` counters on each creature you control": on the creatures `player` controls as it is read, all of them at
/// once (CR 608.2, 101.4: one event, CR 603.2c). The creatures, for what the rest of the sentence does to "those
/// creatures". Abandoned Air Temple, Elspeth, Storm Slayer, Anduril, Narsil Reforged, Gavony Township, Germination
/// Practicum, Ouroboroid and The Theorist, Jace Beleren print it.
pub fn counters_on_creatures_you_control(g: &mut dyn Game, player: PlayerId, kind: CounterKind, n: i16) -> Vec<ObjectId> {
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    counters_on_each(g, &creatures, kind, n);
    creatures
}

/// "Put a loyalty counter on each planeswalker you control": on the planeswalkers `item`'s controller controls as it resolves, all
/// at once (CR 608.2, 101.4: one event; CR 306.5c, 122.1e: loyalty counters). Way of the Mentor, Way of the Necromancer and the Cat
/// token of Worzel, the Protector ("{T}: Put a loyalty counter on each planeswalker you control") print it.
pub fn put_loyalty_counter_on_each_planeswalker_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let walkers: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Planeswalker)).collect();
    counters_on_each(g, &walkers, CounterKind::Loyalty, 1);
    Ok(())
}

/// `n` `kind` counters on each of `objects`, all of them at once (CR 608.2, 101.4).
fn counters_on_each(g: &mut dyn Game, objects: &[ObjectId], kind: CounterKind, n: i16) {
    let puts: Vec<(ObjectId, CounterKind, i16)> = objects.iter().map(|&id| (id, kind, n)).collect();
    g.add_counters_together(&puts);
}

/// "Put `n` `kind` counters on each [subtype] you control": every permanent the ability's controller controls as it resolves
/// that has the subtype (CR 608.2c, 205.3), the source among them, the subtype read through the words of the source's text
/// (CR 612.2); all of them at once (CR 608.2, 101.4: one event, CR 603.2c). The permanents. Lyra, Archangel of Dawn
/// (Angels) and Ajani, Nacatl Avenger (Cats) print it, with one +1/+1 counter: `plus_one_counter_on_each_of_subtype_you_control`.
pub fn counters_on_each_of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, kind: CounterKind, n: i16) -> Vec<ObjectId> {
    of_subtype_you_control(g, item, subtype, false, kind, n)
}

/// "Put `n` `kind` counters on each other [subtype] you control": `counters_on_each_of_subtype_you_control` without the
/// source. Thalia's Lieutenant (Humans), Rescue Retriever (Soldiers), Camellia, the Seedmiser (Squirrels), Agent Phil
/// Coulson and Doctor Spectrum (Heroes) and Vampire Socialite (Vampires) print it, with one +1/+1 counter:
/// `plus_one_counter_on_each_other_of_subtype_you_control`.
pub fn counters_on_each_other_of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, kind: CounterKind, n: i16) -> Vec<ObjectId> {
    of_subtype_you_control(g, item, subtype, true, kind, n)
}

fn of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, other: bool, kind: CounterKind, n: i16) -> Vec<ObjectId> {
    let word = g.words(item.object).subtype(subtype);
    let objects: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| !(other && id == item.object) && g.chars(id).has_subtype(word))
        .collect();
    counters_on_each(g, &objects, kind, n);
    objects
}

/// "Put a +1/+1 counter on each [subtype] you control" as an ability's whole effect, `SUBTYPE` a `Subtype` as `u16`
/// (`lands::sub`): `counters_on_each_of_subtype_you_control`. Lyra, Archangel of Dawn and Ajani, Nacatl Avenger's +2.
pub fn plus_one_counter_on_each_of_subtype_you_control<const SUBTYPE: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_each_of_subtype_you_control(g, item, crate::lands::printed(SUBTYPE), CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put a +1/+1 counter on each other [subtype] you control" as an ability's whole effect: `counters_on_each_other_of_subtype_you_control`.
/// Thalia's Lieutenant, Rescue Retriever, Camellia, the Seedmiser, Agent Phil Coulson, Doctor Spectrum and Vampire Socialite.
pub fn plus_one_counter_on_each_other_of_subtype_you_control<const SUBTYPE: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_each_other_of_subtype_you_control(g, item, crate::lands::printed(SUBTYPE), CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put a +1/+1 counter on [this permanent]" (CR 122.1), the ability's source: none once it has left the battlefield, as a
/// new object it is another (CR 400.7). The whole effect of a trigger or ability of Essence Channeler, Ajani Resolute's
/// Ajani's Pridemate token, Leonardo, Cutting Edge, Blood Hustler, Cryptic Trilobite, Gau, Feral Youth, Hired Claw, Ivora,
/// Insatiable Heir, Laelia, the Blade Reforged, Questing Druid, Shang-Chi and the Ten Rings, Battle-Scarred Survivalist,
/// Tireless Tracker and Youthful Valkyrie.
pub fn plus_one_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put two +1/+1 counters on [this permanent]" (CR 122.1, 201.5): the ability's source, not a new object once it has
/// left the battlefield (CR 400.7). The whole effect of a trigger or ability of Brave Brawler, Demon Wall, Knight of Autumn,
/// Loki Laufeyson, Serpent Specialist, Titanbones, Towering Heart, Viv Vision, Teen Synthezoid, Volcanic Villain and Ygra,
/// Eater of All; one step of the effect of Extremis Elite, Thanos, the Mad Titan, Ultron Drone and Unliving Legionnaire.
pub fn two_plus_one_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 2);
    Ok(())
}

fn creatures_on_the_battlefield(g: &dyn Game) -> Vec<ObjectId> {
    g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect()
}

/// "Destroy all creatures." Every creature goes at once, in battlefield order (CR 701.8a, 101.4); regeneration and
/// indestructible are the verb's business. Day of Judgment prints it.
pub fn destroy_all_creatures(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let creatures = creatures_on_the_battlefield(g);
    g.destroy_all(&creatures);
    Ok(())
}

/// "Destroy all creatures. They can't be regenerated." (CR 701.19c). Wrath of God and Damnation print it.
pub fn destroy_all_creatures_no_regen(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let creatures = creatures_on_the_battlefield(g);
    g.destroy_all_no_regen(&creatures);
    Ok(())
}

/// "Draw N cards" (CR 121.1). Chromatic Star and Mind Stone print "draw a card"; Coveted Jewel and Bankrupt in Blood draw
/// three.
pub fn draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    g.draw(item.controller, n);
    Ok(())
}

/// "You draw a card and lose 1 life" (CR 121.1, 119.3): the controller draws, then loses the life. Plumb the Forbidden,
/// Robot Domination (before its counter) and Elegy Acolyte print it.
pub fn draw_a_card_and_lose_1_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.lose_life(item.controller, 1);
    Ok(())
}

/// "Draw N cards, then put two cards from your hand on top of your library in any order" (CR 121.1, 401.4): the two are
/// chosen together (CR 608.2d) and put back one at a time in the order chosen, so the last chosen ends on top; either may be
/// a card that was already in hand (ruling 2024-06-07), not only one of the N just drawn. Brainsurge (four), Brainstorm and
/// Jace, the Mind Sculptor's 0 ability (three) print it.
pub fn draw_then_put_two_back(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    let player = item.controller;
    g.draw(player, n);
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let cards = crate::choose::select(g, player, item.object, Purpose::PutOnTop, hand, crate::select::cards(crate::select::as_many_as_can(2)))?;
    // The last card chosen ends up on top.
    let order: Vec<ObjectId> = cards.iter().rev().copied().collect();
    g.put_into_library(&order, LibraryPlace::Top);
    Ok(())
}

/// "Target player draws N cards" (CR 121.1): the player the item targeted in `slot`, if the target is still legal (CR 608.2b).
/// Ancestral Recall (three), Flame of Anor and Ashling's Command (two) print it.
pub fn target_player_draws(g: &mut dyn Game, item: &StackItem, slot: usize, n: u32) {
    if let Some(player) = crate::targets::player(g, item, slot) {
        g.draw(player, n);
    }
}

/// "Target player draws N cards, then discards N cards" (CR 121.1, 701.9): the player the item targeted in `slot`, if the target is
/// still legal (CR 608.2b), draws, then chooses that many cards of their hand and discards them. Dack Fayden and Prismari Command
/// print it.
pub fn target_player_draws_then_discards(g: &mut dyn Game, item: &StackItem, slot: usize, n: u32) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, slot) {
        g.draw(player, n);
        crate::discard_chosen(g, player, item.object, n)?;
    }
    Ok(())
}

/// "Draw a card, then discard a card" (CR 121.1, 701.9): the controller draws, then chooses a card of their hand and
/// discards it. Currency Converter and Sphinx of False Conclusions print it.
pub fn loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    crate::discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "Each player draws a card" (CR 121.1, 121.2c): every player still in the game draws one, in turn order from the active
/// player. Faerie Mastermind and Geier Reach Sanitarium print it.
pub fn each_player_draws_a_card(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    for player in g.state().apnap().collect::<Vec<_>>() {
        g.draw(player, 1);
    }
    Ok(())
}

/// "Draw two cards, then discard a card" (CR 121.1, 701.9): the controller draws two, then chooses a card of their hand, the
/// two drawn included, and discards it. Overlord of the Floodpits, Roiling Dragonstorm and Wistfulness print it.
pub fn draw_two_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    crate::discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "You may discard a card. If you do, draw `n` cards" (CR 701.9, 121.1): the controller chooses a card of their hand to
/// discard or none; the draw follows a card that was actually discarded, and nothing is asked of an empty hand. Melded
/// Moxite and Bitter Reunion draw two, Giott, King of the Dwarves and Lorehold, the Historian draw one.
pub fn may_discard_to_draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    if may_discard_a_card(g, item)? {
        g.draw(item.controller, n);
    }
    Ok(())
}

/// "You may discard a card" (CR 701.9): the controller chooses a card of their hand to discard or none, as the ability
/// resolves and not as a cost, so nothing is asked of an empty hand. Whether a card was discarded, for the "if you do" or the
/// reflexive "when you do" (CR 603.12) that follows: a card chosen that its owner can't be made to discard stays (CR 101.2),
/// and then nothing was done. `may_discard_to_draw`, Inti, Seneschal of the Sun and Awaken the Honored Dead use it.
pub fn may_discard_a_card(g: &mut dyn Game, item: &StackItem) -> Result<bool, Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let Some(&card) = crate::choose::cards(g, item.controller, item.object, Purpose::Discard, hand, 1, true)?.first() else { return Ok(false) };
    Ok(!g.discard_together(&[card]).is_empty())
}

/// "You may sacrifice a [quality]" (CR 701.21a): the player chooses, as the effect resolves and not as a cost (CR 608.2d), up to
/// what `rule` allows of their permanents, or none. The engine offers only what the player controls and can sacrifice (CR 101.2),
/// and nothing is asked when none qualifies. Returns what was actually sacrificed, so "if you do" and "when you do" (CR 608.2c,
/// 603.12) are a non-empty check. Tenured Tethermage (a land), Way of the Deathbringer and Grist, the Hunger Tide (a creature), Winter,
/// Tormented Loner (a creature or planeswalker), Entrust the Spark (a planeswalker) and Witherbloom Charm (a permanent) print it.
pub fn may_sacrifice(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection) -> Result<Vec<ObjectId>, Suspend> {
    let permanents = g.permanents_of(player);
    let chosen = crate::choose::select(g, player, source, Purpose::Sacrifice, permanents, rule)?;
    if chosen.is_empty() {
        return Ok(chosen);
    }
    Ok(g.sacrifice(player, &chosen))
}

/// "You may sacrifice an artifact or discard a card. If you do, draw `n` cards" (CR 608.2c-d, 701.9, 701.21). The player chooses
/// as the effect resolves which of the two to do, or neither; an option with nothing to do it with (no artifact, an empty hand)
/// is not offered, and nothing is asked with neither possible. The cards are drawn only if the sacrifice or the discard actually
/// happened. The options are the card's `labels`, in the order "Sacrifice an artifact", "Discard a card", "Neither". Chandra,
/// Spark Hunter (one card) and Vision of Love (two) print it.
pub fn may_sacrifice_an_artifact_or_discard_to_draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    const SACRIFICE: u8 = 0;
    const DISCARD: u8 = 1;
    const NEITHER: u8 = 2;
    let you = item.controller;
    // An artifact that can't be sacrificed is nothing to do it with (CR 101.2: the option can't be taken).
    let artifacts: Vec<ObjectId> =
        g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Artifact) && g.can_sacrifice(you, id)).collect();
    let mut options = Vec::new();
    if !artifacts.is_empty() {
        options.push(SACRIFICE);
    }
    if !g.state().zone(Zone::hand(you)).is_empty() {
        options.push(DISCARD);
    }
    if options.is_empty() {
        return Ok(());
    }
    options.push(NEITHER);
    let card = g.obj(item.object).card;
    let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
    let done = match options[crate::choose::one_of(g, you, item.object, labels)? as usize] {
        SACRIFICE => {
            let chosen = crate::choose::cards(g, you, item.object, Purpose::Sacrifice, artifacts, 1, false)?;
            !g.sacrifice(you, &chosen).is_empty()
        }
        DISCARD => {
            let chosen = crate::choose_from_hand(g, you, item.object, 1)?;
            !g.discard_together(&chosen).is_empty()
        }
        _ => false,
    };
    if done {
        g.draw(you, n);
    }
    Ok(())
}

/// "Discard a card, then draw N cards" (CR 701.9, 121.1): the controller chooses a card of their hand and discards it (nothing is
/// asked of an empty hand), then draws, whether or not they discarded. Fear of Missing Out (one card) and Case of the Crimson
/// Pulse (two) print it.
pub fn discard_then_draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    crate::discard_chosen(g, item.controller, item.object, 1)?;
    g.draw(item.controller, n);
    Ok(())
}

/// "Discard your hand, then draw N cards" (CR 701.9, 121.1): the whole hand goes at once with no choice, then the draw
/// happens whatever the hand was. Anjes Ravager (three cards) and Case of the Crimson Pulse (two) print it.
pub fn discard_hand_then_draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    g.discard_hand(item.controller);
    g.draw(item.controller, n);
    Ok(())
}

/// "Each player discards their hand, then draws seven cards": every player, in turn order from the active player
/// (CR 101.4), discards together and then draws; nobody is asked, a whole hand leaving nothing to choose. Wheel of Fortune
/// and Reforge the Soul print it.
pub fn each_player_discards_hand_then_draws_seven(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    players_discard_hands_then_draw_seven(g, &players);
    Ok(())
}

/// The discard-and-draw of the wheel for the given `players` (listed in turn order from the active player, CR 101.4): their
/// hands go together (CR 701.9a), then each draws seven in that order (CR 121.2c). Wheel of Fortune and Reforge the Soul pass
/// every player; Arc of Fortune passes those who chose it.
pub fn players_discard_hands_then_draw_seven(g: &mut dyn Game, players: &[PlayerId]) {
    let hands: Vec<ObjectId> = players.iter().flat_map(|&p| g.state().zone(Zone::hand(p)).to_vec()).collect();
    g.discard_together(&hands);
    for &p in players {
        g.draw(p, 7);
    }
}

/// "You gain N life" (CR 119.3). Zuran Orb and Inventor's Fair print it (two and one).
pub fn gain_life(g: &mut dyn Game, item: &StackItem, n: i32) -> Result<(), Suspend> {
    g.gain_life(item.controller, n);
    Ok(())
}

/// "You gain 1 life" as a triggered or activated ability. Koth of the Homestead and Medic's Kitesail use this callback.
pub fn gain_one_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, 1)
}

/// "Each opponent loses 1 life and you gain 1 life" (CR 119.3): every opponent still in the game loses it together, and the
/// ability's controller gains 1. Bontus's Monument, Doom Reigns Supreme, Grave Venerations, Raven of Fell Omens, Thunder of
/// Unity, Tithing Blade, Witherbloom Apprentice and Dream Beavers print it.
pub fn each_opponent_loses_1_life_you_gain_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 1);
    g.gain_life(item.controller, 1);
    Ok(())
}

/// "Each opponent loses 1 life" (CR 119.3): every opponent still in the game loses it together. Lunar Convocation, Starscape
/// Cleric and The Meathook Massacre print it.
pub fn each_opponent_loses_1_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 1);
    Ok(())
}

/// "Target player loses `n` life and you gain `n` life" (CR 119.3), and "target opponent ...": the first target, a player,
/// whichever its slot allows. A target no longer legal loses nothing (CR 608.2b), while the gain, which does not use it, is
/// still made when the resolution goes on without it (a mode of a modal spell whose other target is legal); an ability with that
/// target alone never resolves. Collective Brutality and Witherbloom Command print it with 2 of a target opponent, and
/// `target_player_loses_1_life_you_gain_1` of 1.
pub fn target_player_loses_life_you_gain_life(g: &mut dyn Game, item: &StackItem, n: i32) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Player(player) = item.target(0)
    {
        g.lose_life(player, n);
    }
    g.gain_life(item.controller, n);
    Ok(())
}

/// "Target player loses 1 life and you gain 1 life" (CR 119.3), and "target opponent ...": `target_player_loses_life_you_gain_life`
/// of 1. Blood Artist prints it of a player; Blood Hustler and Down, Down to Goblin-town of an opponent.
pub fn target_player_loses_1_life_you_gain_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    target_player_loses_life_you_gain_life(g, item, 1)
}

/// "[This source] deals `n` damage to each opponent" (CR 120.3, 102.2): one damage event for each opponent still in the game,
/// dealt together (CR 120.2), by the resolving object as it last was if it has left (CR 113.7a). Chandra, Torch of Defiance,
/// Eye of Jace, Fiery Confluence, Impact Tremors, Warleader's Call, Vivi Ornitier, Gau, Keral Keep Disciples, Voldaren
/// Epicure and Quintorius Kand print it.
pub fn damage_each_opponent(g: &mut dyn Game, item: &StackItem, n: u16) {
    let hits: Vec<(ObjectId, Target, u16)> = g.state().opponents(item.controller).map(|p| (item.object, Target::Player(p), n)).collect();
    g.damage_together(&hits);
}

/// "This source deals 1 damage to that creature's controller", the creature being the trigger's subject: its controller
/// as the ability resolves, or as it last existed if it has left the battlefield (CR 608.2h), with no target and no
/// retest of the trigger condition. The damage is dealt by the ability's source, as it last existed if it has left
/// (CR 113.7a); none is dealt to a player who has left the game (CR 102.1, 120.1, 800.4a), which the core's damage sees to. Rampaging
/// Ferocidon and Tetsuko Umezawa, Pursuer print it.
pub fn ping_subject_controller(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = item.subject.object() {
        let controller = g.controller(creature);
        g.damage(item.object, Target::Player(controller), 1);
    }
    Ok(())
}

/// "Surveil N" (CR 701.25) for the ability's controller. The surveil lands and Dragon's Rage Channeler print it.
pub fn surveil(g: &mut dyn Game, item: &StackItem, n: usize) -> Result<(), Suspend> {
    crate::surveil(g, item.controller, item.object, n)
}

/// "Scry N" (CR 701.22) for the ability's controller. The scry lands (Crystal Grotto, Temple of Deceit) print it.
pub fn scry(g: &mut dyn Game, item: &StackItem, n: usize) -> Result<(), Suspend> {
    crate::scry(g, item.controller, item.object, n)
}

/// "Scry 2" (CR 701.22) for the ability's controller, as a function the ability takes whole. Candy Trail, Giant's Boulder,
/// Oteclan Landmark, Planetarium of Wan Shi Tong, Simulacrum Synthesizer, Stormwing Entity and Guardian of New Benalia print it.
pub fn scry_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    scry(g, item, 2)
}

/// "You become the monarch" (CR 725.1). Aragorn, King of Gondor and Court of Garenbrig print it.
pub fn become_monarch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.become_monarch(item.controller);
    Ok(())
}

/// "You take the initiative" (CR 726.2). Passageway Seer and Seasoned Dungeoneer print it.
pub fn take_the_initiative(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.take_initiative(item.controller);
    Ok(())
}

/// "Put a loyalty counter on [this planeswalker]" (CR 122.1, 306.5b): the ability's source, if it is still the permanent that
/// triggered. Ajani Resolute, Tezzeret, Cruel Captain and Ral, Crackling Wit print it.
pub fn put_loyalty_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Loyalty, 1);
    Ok(())
}

/// "Add one mana of any color" (CR 106.1b, 605.1b): the color is chosen by the ability's controller as it resolves; an ability
/// that adds mana and is not a mana ability goes on the stack. Lotus Cobra, The Legend of Roku and Outcaster Trailblazer print it.
pub fn add_one_mana_of_any_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = crate::choose_color(g, item.controller, item.object)?;
    g.add_mana(item.controller, color.mana(), 1);
    Ok(())
}

/// "Each opponent discards a card" (CR 701.9, 101.4): each opponent still in the game chooses in turn order from the active player (APNAP) and the cards are
/// discarded together; one with no card in hand does not discard. Rank Rat, Aclazotz, Deepest Betrayal and Hopeless Nightmare
/// print it.
pub fn each_opponent_discards_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
    crate::each_discards_a_card(g, &opponents, item.object)?;
    Ok(())
}

/// "Target player discards N cards": their choice (CR 701.9), the player being the first target. Nothing if that player is
/// no longer a legal target as it resolves (CR 608.2b): a modal spell whose other mode's target is still legal resolves
/// without it (Kolaghan's Command). Demonic Pact ("target opponent"), Kolaghan's Command and Armus, Skin of Evil print it.
pub fn target_player_discards<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        crate::discard_chosen(g, player, item.object, N)?;
    }
    Ok(())
}

/// "Sacrifice this [permanent]" (CR 701.21a): the ability's source, sacrificed by the ability's controller - by no one when
/// another player has gained control of it since. City of Traitors and Carnage, Crimson Chaos print it.
pub fn sacrifice_self(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

/// "The next spell you cast this turn can't be countered" (CR 611.2c, 601.2, 701.5a): an effect about the ability's
/// controller, which marks the next spell that player casts this turn as it is cast, before anyone has priority, and ends
/// with the turn if they cast none. Each activation is its own effect, so two before the next spell both apply to it.
/// Mistrise Village and Theorist's Proxy print it.
pub fn next_spell_cant_be_countered(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(item.controller, item.object, EffectKind::NextSpellCantBeCountered, Duration::EndOfTurn);
    Ok(())
}

/// "Destroy target [thing]" (CR 701.8): the ability's first target.
pub fn destroy_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::destroy_target(g, item, 0);
    Ok(())
}

/// "Destroy target [permanent]. Its controller creates a [token]" (CR 701.8, 608.2h): the ability's first target, its
/// controller read before the destroy, and the token created whether or not it was destroyed (indestructible, regenerated);
/// an illegal target creates nothing (CR 608.2b). Stroke of Midnight and Bovine Intervention print it.
pub fn destroy_and_its_controller_creates(g: &mut dyn Game, item: &StackItem, token: &'static CardDef) -> Result<(), Suspend> {
    let Some(target) = crate::targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(target);
    g.destroy(target);
    crate::tokens::create(g, controller, token, 1)?;
    Ok(())
}

/// "{1}, Sacrifice this creature: Destroy target artifact or enchantment." Cathar Commando, Outland Liberator
/// (and its back face) and Shattered Acolyte print it.
pub const SACRIFICE_TO_DESTROY_ARTIFACT_OR_ENCHANTMENT: AbilityDef = ability(
    cost("{1}").and(SACRIFICE_THIS),
    destroy_first_target,
    "{1}, Sacrifice this creature: Destroy target artifact or enchantment.",
)
.targets(&[crate::targets::ARTIFACT_OR_ENCHANTMENT]);

/// "Transform this [permanent]" (CR 701.27a): the ability's source, which does nothing if it has transformed since the
/// ability was put on the stack or, for a delayed ability, created (CR 701.27f). Aang, Swift Savior, Concealing Curtains,
/// Norman Osborn, Temple of the Dead and Aang, at the Crossroads's delayed ability print it.
pub fn transform_self(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.transform_by(item.object, item)
}

/// "When [this] dies, return it to the battlefield tapped and transformed under its owner's control" (CR 712.14a): the card
/// it became in the graveyard, if it is still there (CR 400.7e); a permanent that is not a double-faced card (a copy of one)
/// has no back face and stays in the graveyard. The trigger looks back (CR 603.10a). Aclazotz, Deepest Betrayal and Ojer
/// Axonil, Deepest Might print it.
pub fn return_it_tapped_and_transformed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = g.found_after_move(item.object) else { return Ok(()) };
    let owner = g.obj(card).owner;
    g.put_onto_battlefield_transformed(card, owner, true)?;
    Ok(())
}

/// "Untap this [permanent]" (CR 701.26): the ability's source. Basalt Monolith and Grim Monolith print it.
pub fn untap_self(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.untap(item.object);
    Ok(())
}

/// "Tap [N] target [permanents]" (CR 701.26a, 608.2b): each target that is still legal as the ability resolves. Divining
/// Duelist (as a mode), Quake, Agent of S.H.I.E.L.D. and Raft Security Officer print it.
pub fn tap_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for permanent in crate::targets::legal_objects(g, item) {
        g.tap(permanent);
    }
    Ok(())
}

/// "Untap [N] target [permanents]" (CR 701.26, 608.2b): each target that is still legal as the ability resolves. Argothian
/// Elder, Candelabra of Tawnos and Garruk, Curse Breaker (for lands), Formidable Speaker, Kiora, Behemoth Beckoner, Voltaic Key, Manifold Key, Earthcraft, Hapatra, the Desert Frost and Deceiver Exarch and Divining Duelist (as a mode) print it.
pub fn untap_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for permanent in crate::targets::legal_objects(g, item) {
        g.untap(permanent);
    }
    Ok(())
}

/// "You may [do something]" (CR 117.3, 609.3): asks `player` whether to, for `purpose`, and says whether they did. The
/// question is the usual `May`, so a scenario answers it with `Accept` or `Decline`. Ad Nauseam, Cori-Steel Cutter and Currency Converter ask
/// it.
pub fn ask_may(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose) -> Result<bool, Suspend> {
    Ok(g.ask(player, RequestKind::May { source, purpose })? == Answer::Yes)
}

/// "[Player] may cast [card] without paying its mana cost" as the effect resolves (CR 608.2g, 118.9): `player` casts it then
/// or not at all, its timing waived, X 0 (CR 107.3b), its additional costs paid. Nothing is asked when they could not cast
/// it at all (a land, no legal target for them, split second on the stack); asked, they may decline, and a cast they give
/// up (CR 733.1) leaves it where it was. `player` is the effect's controller or the player it names (`CastTerms::caster`:
/// the card's owner, "that player"). Whether it was cast. Spell Queller (its owner), Ore-Rich Stalactite and the hideaway
/// lands' "play the exiled card" print it.
pub fn may_cast_without_paying(g: &mut dyn Game, player: PlayerId, card: ObjectId) -> Result<bool, Suspend> {
    if !g.could_cast_during_resolution_by(card, player) || !ask_may(g, player, card, Purpose::CastFree)? {
        return Ok(false);
    }
    g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), caster: Some(player), ..CastTerms::default() })
}

/// "You may draw `n` cards" (CR 121.3, 121.2b): not offered when `player` can't draw that many ("can't draw more than one
/// card each turn" forbids a choice of two even before any card is drawn, Narset, Parter of Veils' ruling); asked with
/// `question`, and the cards drawn on a yes. Whether they chose to draw. Leovold, Emissary of Trest, Consecrated Sphinx and
/// Sylvan Library print it.
pub fn may_draw(g: &mut dyn Game, player: PlayerId, source: ObjectId, n: u32, question: &'static Question) -> Result<bool, Suspend> {
    if cant_draw(g, player, n) || !ask_may(g, player, source, Purpose::Ask(question))? {
        return Ok(false);
    }
    g.draw(player, n);
    Ok(true)
}

const MAY_LOOT: Question = Question::new("Draw", "draw a card?", "If you do, discard a card", "Draw and discard", "Don't draw");

/// "You may draw a card. If you do, discard a card" (CR 121.3, 608.2c): the choice is not offered a player who can't draw
/// (`may_draw`), and "if you do" is whether a card was drawn, not whether the player said yes, so a draw that a replacement
/// took (CR 614.6) or an empty library made discards nothing. Smuggler's Copter, Jeskai Ascendancy and Duelist of the Mind
/// print it.
pub fn may_draw_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let before = g.history().drawn(player);
    if may_draw(g, player, item.object, 1, &MAY_LOOT)? && g.history().drawn(player) > before {
        crate::discard_chosen(g, player, item.object, 1)?;
    }
    Ok(())
}

const MAY_MILL: Question = Question::new("Mill", "mill a card?", "The top card of your library goes into your graveyard", "Mill", "Don't mill");

/// "You may mill a card" (CR 701.17a): asked of the ability's controller, then the mill. Mole Man's Moloid tokens and
/// Patchwork Beastie print it.
pub fn may_mill_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_MILL))? {
        g.mill(item.controller, 1);
    }
    Ok(())
}

/// "You may return this card from your graveyard to ..." (CR 113.6k): `ask_may` for a card that triggered from a zone it
/// functions in, asked only while the card is still the object that triggered (CR 400.7). Once it has left, nothing is asked
/// and the answer is no. Squee, Goblin Nabob and Vengevine ask it.
pub fn ask_may_return_this_card(g: &mut dyn Game, item: &StackItem, purpose: Purpose) -> Result<bool, Suspend> {
    if !is_live(g, item.object) {
        return Ok(false);
    }
    ask_may(g, item.controller, item.object, purpose)
}

const RETURN_TO_BATTLEFIELD: Question = Question::new("ReturnToBattlefield", "return it from your graveyard to the battlefield?", "", "Return it", "Leave it");

/// "You may return this card from your graveyard to the battlefield" (CR 113.6k): `ask_may_return_this_card` with the
/// question worded for the battlefield, and the card put onto the battlefield untapped under its controller on a yes.
/// Nothing is asked once it is no longer the object that triggered (CR 400.7). Vengevine and Bloodghast print it.
pub fn may_return_this_card_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_return_this_card(g, item, Purpose::Ask(&RETURN_TO_BATTLEFIELD))? {
        g.put_onto_battlefield(item.object, item.controller, false)?;
    }
    Ok(())
}

/// "Return this card from your graveyard to the battlefield" (CR 400.7): the ability's own source, put onto the
/// battlefield untapped under its controller; nothing if it is no longer the object the ability came from (the core skips a
/// card that changed zones). Darklight Phoenix and Cauldron Familiar print it.
pub fn return_this_card_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_onto_battlefield(item.object, item.controller, false)?;
    Ok(())
}

/// "Return this [permanent or card] to its owner's hand" (CR 400.3): the ability's own source, from the battlefield or a
/// graveyard. Nothing if it is no longer the object it was by the time the ability resolves (CR 400.7). Altar of the Wretched
/// and Puppet Crafting ("from your graveyard") and Bramble Familiar ("this creature") print it.
pub fn return_this_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.return_to_hand(&[item.object]);
    Ok(())
}

/// "Return target spell or permanent to its owner's hand": a spell leaves the stack for its owner's hand without being
/// countered (CR 400.3, so one that can't be countered goes too); a permanent is returned (CR 400.7: a token ceases to
/// exist, CR 111.7). Which it is is read from the stack. Jeskai Revelation and Sink into Stupor print it.
pub fn return_spell_or_permanent_to_hand(g: &mut dyn Game, object: ObjectId) {
    if g.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == object) {
        g.remove_spell(object, CounterTo::Hand);
    } else {
        g.return_to_hand(&[object]);
    }
}

/// "Return target [permanent] to its owner's hand" (CR 400.3): the ability's or
/// spell's first target, nothing at all if it is no longer a legal target (CR 608.2b). A token that is returned ceases to
/// exist (CR 111.7). Aether Spellbomb, Banishing Knack, Karakas, Mystic Confluence, Brazen Borrower's Petty Theft and
/// Bounce Off print it, and, for a card in a graveyard, Bala Ged Recovery, Emeritus of Abundance and Auriok Salvagers.
pub fn return_first_target_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = crate::targets::object(g, item, 0) {
        g.return_to_hand(&[permanent]);
    }
    Ok(())
}

/// "Exile target [permanent]", remembered as exiled with the ability's source (CR 607.2a), so that its linked ability
/// ("return the exiled card") finds it; nothing if the target is no longer legal (CR 608.2b). The card stays in exile
/// whatever happens to the source: it is not "until [this] leaves the battlefield" (CR 610.3). Oblivion Ring, Parallax
/// Wave and Skyclave Apparition print it.
pub fn exile_first_target_with_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = crate::targets::object(g, item, 0) {
        g.exile_with(&[permanent], g.linked(item), false);
    }
    Ok(())
}

/// "Return the exiled card(s) to the battlefield under [their] owner's control" (CR 607.2a, 110.2a): every card exiled
/// with the ability's source, together (CR 101.4), each under its owner's control; an Aura among them chooses what it
/// enchants as it enters (CR 303.4f). The cards are new objects (CR 400.7). Oblivion Ring, Worldgorger Dragon, Parallax
/// Wave ("each player returns ... all cards they own exiled with it") and Touch the Spirit Realm print it.
pub fn return_exiled_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId)> = g.exiled_with(g.linked(item)).into_iter().map(|card| (card, g.obj(card).owner)).collect();
    if !cards.is_empty() {
        g.put_all_onto_battlefield(&cards)?;
    }
    Ok(())
}

/// "Return target [card] from your graveyard to the battlefield" (CR 400.3): the first target, under the controller of the
/// spell or ability, untapped; nothing if it is no longer a legal target (CR 608.2b), which for a lone required target
/// means the spell or ability does not resolve at all. Emptiness and Ral Zarek, Guest Lecturer's −2 print it.
pub fn return_first_target_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = crate::targets::object(g, item, 0) {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

/// "[This source] deals `damage` damage to [a target] and you gain `life` life" (CR 120.3, 119.1): the first target, if it is
/// still legal (CR 608.2b), is dealt the damage by the resolving object (as it last was if it has left, CR 113.7a). The gain,
/// a fixed amount that does not use the target, is made whenever this resolves: an item with that target alone never resolves
/// once it is illegal, and a mode of a modal spell still resolving for another mode's target gains it anyway (CR 608.2b), as
/// `target_player_loses_life_you_gain_life` does. Which targets are allowed is the card's own spell or ability. Lightning
/// Helix, Phlage, Titan of Fire's Fury and Demonic Pact print it with equal amounts; Knockout Blow deals 4 and gains 2.
pub fn damage_target_and_gain_life(g: &mut dyn Game, item: &StackItem, damage: u16, life: i32) -> Result<(), Suspend> {
    if g.target_legal(item, 0) {
        g.damage(item.object, item.target(0), damage);
    }
    g.gain_life(item.controller, life);
    Ok(())
}

/// "Change the target of target spell or ability with a single target" (CR 115.7a): the first target of the resolving spell or
/// ability is a spell or ability on the stack whose target its controller changes to another legal one as this resolves, and
/// the target is not changed if there is no other legal choice. A target no longer legal is not changed (CR 608.2b): a mode of a
/// modal spell still resolves for another mode's legal target (Return the Favor). Untimely Malfunction and Bolt Bend print it.
pub fn change_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.target_legal(item, 0) {
        return Ok(());
    }
    g.change_target(item.target(0), item.controller)
}

/// "[This spell] deals `n` damage to each creature" (Pyroclasm, Fiery Confluence, Avengers Disassembled): one damage event
/// for every creature on the battlefield as the effect is carried out (CR 608.2c), all dealt together.
pub fn damage_each_creature(g: &mut dyn Game, item: &StackItem, n: u16) {
    damage_each_creature_dealt(g, item, n);
}

/// "[This spell] deals `n` damage to each non-[subtype] creature" (CR 608.2c, 120.2): one damage event for every creature on
/// the battlefield without `subtype` as the effect is carried out, all dealt together. The caller reads the subtype through
/// the words of the object whose text names it (`g.words(item.object).subtype(..)`, CR 612). Desolation of Smaug (non-Dragon)
/// and Chandra, Awakened Inferno (non-Elemental) print it.
pub fn damage_each_creature_not_of_subtype(g: &mut dyn Game, item: &StackItem, subtype: Subtype, n: u16) {
    let hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) && !chars.has_subtype(subtype)
        })
        .map(|id| (item.object, Target::Object(id), n))
        .collect();
    g.damage_together(&hits);
}

/// "[This spell] deals `n` damage to each creature and each planeswalker" (Brotherhood's End, Storm's Wrath, Burn Down the
/// House, Temporal Firestorm, Dragonback Assault, Invasion of Karsus, Star of Extinction, Underworld Fires, Calamitous
/// Cave-In): one damage event for every creature and planeswalker on the battlefield as the effect is carried out (CR 608.2c),
/// each permanent once even when it is both, all dealt together. Damage to a planeswalker removes loyalty counters (CR 306.8).
pub fn damage_each_creature_and_planeswalker(g: &mut dyn Game, item: &StackItem, n: u16) {
    let hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) || chars.is(CardType::Planeswalker)
        })
        .map(|id| (item.object, Target::Object(id), n))
        .collect();
    g.damage_together(&hits);
}

/// `damage_each_creature`, answering which creatures were actually dealt damage by it (CR 120.3), for "a creature dealt
/// damage this way" (Anger of the Gods, Pyroclasm's kin). They are read from the damage events the resolution recorded, so a
/// creature whose damage was prevented is not among them and one a redirection sent the damage to (CR 614.9, Ancient
/// Adamantoise) is, once however many times it was dealt damage.
pub fn damage_each_creature_dealt(g: &mut dyn Game, item: &StackItem, n: u16) -> Vec<ObjectId> {
    let hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (item.object, Target::Object(id), n))
        .collect();
    let before = g.history().head() as usize;
    g.damage_together(&hits);
    let mut dealt = crate::history::permanents_dealt_damage_by(g.history(), item.object, before);
    dealt.retain(|&id| g.chars(id).is(CardType::Creature));
    dealt
}

/// "[This] deals `n` damage to that player": the player a trigger fired about (`Fired::about(Target::Player(..))`, its
/// subject), dealt by the ability's source, as it last was if it has left (CR 113.7a, 608.2h). Burning-Tree Shaman, Harsh
/// Mentor and Immolation Shaman print it.
pub fn damage_that_player(g: &mut dyn Game, item: &StackItem, n: u16) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.damage(item.object, Target::Player(p), n);
    }
    Ok(())
}

/// "`target` gains hexproof until end of turn" (CR 702.11b, 611.2a): a layer 6 keyword for the rest of the turn, from the
/// effect's source. Liliana the Faultless, Octopus Form, Shore Up and Snakeskin Veil print it.
pub fn gain_hexproof_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId) {
    gain_until_end_of_turn(g, source, target, Keyword::Hexproof);
}

/// "`target` gains lifelink until end of turn" (CR 702.15, 611.2a): a layer 6 keyword for the rest of the turn, from the
/// effect's source. Gix's Command, Archenemy's Charm, Hardened Academic and Obsessive Pursuit print it.
pub fn gain_lifelink_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId) {
    gain_until_end_of_turn(g, source, target, Keyword::Lifelink);
}

/// "It gains haste" with no duration (CR 611.2a, 613.1f): a layer 6 keyword for as long as the object stays the object it is
/// (a new one, CR 400.7, has none). Anzrag's Rampage, Sneak Attack, Through the Breach, Emperor of Bones, Goryo's Vengeance,
/// Earthbend and unearth print it.
pub fn gain_haste(g: &mut dyn Game, source: ObjectId, object: ObjectId) {
    g.add_effect(source, object, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), Duration::Indefinite);
}

/// "`target` gains indestructible until end of turn" (CR 702.12b, 611.2a): a layer 6 keyword for the rest of the turn, from the
/// effect's source. Arwen, Mortal Queen, Cecil, Dark Knight, Crumb and Get It, Divine Resilience, Iron Shield Elf, Revitalizing Repast, Ursine
/// Monstrosity and Boros Charm print it.
pub fn gain_indestructible_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId) {
    gain_until_end_of_turn(g, source, target, Keyword::Indestructible);
}

/// "This creature gains indestructible until end of turn. Tap it." (CR 702.12b, 611.2a, 701.26a): if the ability's source is
/// still on the battlefield as the permanent it was (CR 400.7), it gains the keyword for the rest of the turn, then it is
/// tapped. The tap is the ability's own instruction, not its cost, so the ability can be activated while it is tapped
/// already (Underrealm Lich's ruling). Iron-Shield Elf and Underrealm Lich print it.
pub fn this_gains_indestructible_then_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creature = item.object;
    if crate::targets::permanent(g, creature) {
        gain_indestructible_until_end_of_turn(g, creature, creature);
        g.tap(creature);
    }
    Ok(())
}

/// "`target` becomes a [creature type] in addition to its other types until end of turn" (CR 205.1b, 611.2a, 613.1d): a layer 4
/// effect from `source` that adds the subtype, read through the words of the source's text (CR 612) so a text-changing effect
/// changes it. Sarkhan, Dragon Ascendant and Enter the Avatar State print it.
pub fn becomes_subtype_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId, subtype: Subtype) {
    let subtype = g.words(source).subtype(subtype);
    g.add_effect(source, target, EffectKind::AddSubtype(subtype), Duration::EndOfTurn);
}

/// "`target` gains [a keyword] until end of turn" (CR 611.2a): a layer 6 keyword for the rest of the turn, from the effect's
/// source. Creeping Tar Pit, Immolation Shaman and Kappa Cannoneer print it inside a longer instruction ("can't be blocked
/// this turn", "gains menace"); the `this_gains_*` below are the abilities that only give it.
pub fn gain_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId, keyword: Keyword) {
    g.add_effect(source, target, EffectKind::AddKeywords(Keywords::of(&[keyword])), Duration::EndOfTurn);
}

/// "This creature gains [a keyword] until end of turn" (CR 611.2a): the ability's source gains the keyword for the rest of
/// the turn; an effect on a source that is no longer the object it was does nothing (CR 400.7). Afterthought Sentry, Psychic Frog and Spectacular Spider-Man
/// ("flying"), Fleeting Spirit ("first strike") and Poison Dart Frog ("deathtouch") print it; the named wrappers below are
/// the fn pointers an ability's effect takes.
pub fn this_gains_until_end_of_turn(g: &mut dyn Game, item: &StackItem, keyword: Keyword) -> Result<(), Suspend> {
    gain_until_end_of_turn(g, item.object, item.object, keyword);
    Ok(())
}

/// "This creature gains flying until end of turn" (CR 702.9b): `this_gains_until_end_of_turn` for flying.
pub fn this_gains_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::Flying)
}

/// "This creature gains first strike until end of turn" (CR 702.7a): `this_gains_until_end_of_turn` for first strike.
pub fn this_gains_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::FirstStrike)
}

/// "This creature gains deathtouch until end of turn" (CR 702.2b): `this_gains_until_end_of_turn` for deathtouch.
pub fn this_gains_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::Deathtouch)
}

/// "This creature gains indestructible until end of turn" (CR 702.12b): `this_gains_until_end_of_turn` for indestructible.
/// Belligerent Regisaur prints it.
pub fn this_gains_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::Indestructible)
}

/// "`target` gains hexproof and indestructible until end of turn" (CR 702.11b, 702.12b, 611.2a): both keywords from one
/// effect, a layer 6 change for the rest of the turn. Invasion of Gobakhan's Lightshield Array, Spectacular Spider-Man,
/// Restoration Magic and Defend the Rider print it.
pub fn gain_hexproof_and_indestructible_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId) {
    g.add_effect(source, target, EffectKind::AddKeywords(Keywords::of(&[Keyword::Hexproof, Keyword::Indestructible])), Duration::EndOfTurn);
}

/// "A creature or planeswalker with the greatest mana value among creatures and planeswalkers they control": the
/// permanents `player` controls now that tie for the greatest mana value (`Query::mana_value`: tokens, copies and face-down
/// permanents read as the rules say) in the one group of their creatures and planeswalkers, so a planeswalker of 5 outranks a
/// creature of 3. The player chooses among them (CR 701.21a, 608.2d). Break Under Pressure sacrifices one of them; End of
/// the Hunt exiles one.
pub fn greatest_mana_value_creature_or_planeswalker(g: &dyn Game, player: PlayerId) -> Vec<ObjectId> {
    let group: Vec<ObjectId> = g
        .permanents_of(player)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.is(CardType::Creature) || chars.is(CardType::Planeswalker)
        })
        .collect();
    let greatest = group.iter().map(|&id| g.mana_value(id)).max().unwrap_or(0);
    group.into_iter().filter(|&id| g.mana_value(id) == greatest).collect()
}

/// "Exile each opponent's graveyard" (CR 406.2, 608.2f): every card in the graveyard of each opponent of `controller` still in
/// the game (CR 800.4a), in one move. Soul-Guide Lantern and Dauntless Scrapbot print it.
pub fn exile_each_opponents_graveyard(g: &mut dyn Game, controller: PlayerId) {
    let mut cards = Vec::new();
    for opponent in g.state().opponents(controller).collect::<Vec<_>>() {
        cards.extend_from_slice(g.state().zone(Zone::graveyard(opponent)));
    }
    g.exile_all(&cards);
}

/// "Exile target player's graveyard" (CR 406.2, 701.13): every card in the graveyard of the ability's first target player in
/// one move; nothing if that player is no longer a legal target (CR 608.2b). Kutzil's Flanker and Tormod's Crypt print it.
pub fn exile_first_target_players_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        let cards = g.state().zone(Zone::graveyard(player)).to_vec();
        g.exile_all(&cards);
    }
    Ok(())
}

/// "Exile all creatures" (CR 406.2, 608.2f): every creature on the battlefield, whoever controls it, in one move, tokens too.
/// How many were exiled (CR 608.2c: the moves made, those a replacement sent elsewhere included), for "X is the number of
/// creatures exiled this way". Sunfall and Overwrite the Multiverse print it.
pub fn exile_all_creatures(g: &mut dyn Game) -> usize {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.exile_all(&creatures).len()
}

/// "Exile all graveyards" (CR 406.2): every card in every graveyard, each to its owner's exile zone, in one move (CR 608.2c).
/// Rest in Peace and Farewell print it.
pub fn exile_all_graveyards(g: &mut dyn Game) {
    let mut cards: Vec<ObjectId> = Vec::new();
    for owner in g.state().seats() {
        cards.extend(g.state().zone(Zone::graveyard(owner)).iter().copied());
    }
    g.exile_all(&cards);
}

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn." (CR 613.1b,
/// 611.2a, 702.10): the resolving spell's first target, if it is still legal (CR 608.2b), which it returns. Act of Treason
/// and Vengeful Possession print it (Threaten, Traitorous Blood with trample too); one the caster already controls is
/// untapped and given haste all the same (Act of Treason's ruling).
pub fn gain_control_untap_haste(g: &mut dyn Game, item: &StackItem) -> Option<ObjectId> {
    let creature = crate::targets::object(g, item, 0)?;
    g.gain_control(item.object, creature, item.controller, Duration::EndOfTurn);
    // Three sentences, each judged by the state triggers before the next (CR 603.8, 608.2c).
    g.end_sentence();
    g.untap(creature);
    g.end_sentence();
    crate::haste_until_end_of_turn(g, item.object, creature);
    Some(creature)
}

const TRANSFORM: Question = Question::new("Transform", "pay {cost} to transform {subject}?", "", "Pay {cost}", "Don't transform");

/// "You may pay [cost]. If you do, transform [this]." Asked as the ability resolves, wherever the permanent is: paying is
/// the choice, and it transforms only if it is still the permanent the ability is of and it hasn't transformed since the
/// ability was put onto the stack (CR 701.27f). The mana paid is no spell's, so mana for spells can't pay it (CR 106.6).
/// Ashling, Rekindled and Sygg, Wanderwine Wisdom print it.
pub fn may_pay_to_transform(g: &mut dyn Game, item: &StackItem, cost: ManaCost) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, of_mana(cost), &TRANSFORM)? {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// "Until end of turn, target creature loses all abilities and becomes a [color] [creature type] with base power and
/// toughness [P]/[T]" (Turn to Frog, Turn // Burn's Turn), for the target in `slot`: continuous effects from a resolved
/// spell (CR 611.2), each in its layer and all at this spell's timestamp (CR 613.7a) - layer 4, its creature types replaced
/// by the one named, its card types and supertypes kept (CR 205.1a, the rulings: an artifact creature stays one); layer 5,
/// that color and no other (CR 105.3); layer 6, every ability it has by then lost, and one gained later kept (CR 613.7, the
/// rulings); layer 7b, the base power and toughness, over every earlier base-setting effect and under a later one, and
/// under whatever modifies them, counters included (layer 7c). Abilities that already triggered or were activated still
/// resolve (the rulings). It lasts until the cleanup step (CR 514.2). The color and the creature type are words of the
/// spell's text (CR 612), read through its words. With the target gone or no longer a creature, nothing happens (CR 608.2b).
pub fn loses_all_abilities_and_becomes(g: &mut dyn Game, item: &StackItem, slot: usize, color: Color, kind: Subtype, power: i16, toughness: i16) {
    let Some(creature) = crate::targets::object(g, item, slot) else { return };
    let words = g.words(item.object);
    let source = item.object;
    // One instruction (CR 608.2c): the state triggers see it all done.
    instruction(g, |g| {
        g.add_effect(source, creature, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
        g.add_effect(source, creature, EffectKind::SetCreatureType(words.subtype(kind)), Duration::EndOfTurn);
        g.add_effect(source, creature, EffectKind::SetColors(words.colors(ColorSet::of(color))), Duration::EndOfTurn);
        g.add_effect(source, creature, EffectKind::SetPt(power, toughness), Duration::EndOfTurn);
    });
}

/// A Gideon's "Until end of turn, Gideon becomes a P/T [colors] [creature types] creature [with keywords] that's still a
/// planeswalker. Prevent all damage that would be dealt to him this turn." (CR 205.1b, 611.2a, 613.1d, 615.1): the
/// animation is laid over him until the cleanup step as a manland's is (`animate`: the creature type, the colors where the
/// text names some, the keywords, the base power and toughness), the creature types and colors read through his words (CR
/// 612); then, in its own sentence, all damage that would be dealt to him this turn is prevented, by any source, combat or
/// not. Neither happens to a new object, should he have left (CR 400.7). Gideon, Battle-Forged and Gideon of the Trials
/// (a 4/4 Human Soldier with indestructible), Gideon, Martial Paragon (5/5), Gideon, Ally of Zendikar (a 5/5 Human Soldier
/// Ally), Gideon Jura (a 6/6 Human Soldier, no keyword) and Gideon, the Oathsworn (a 5/5 white Soldier) print it. The card
/// lists `prevention::ANY_DAMAGE` `index`th in its `granted_preventions`.
pub fn planeswalker_becomes_creature_preventing_damage(
    g: &mut dyn Game,
    item: &StackItem,
    (power, toughness): (i16, i16),
    colors: Option<ColorSet>,
    subtypes: &[Subtype],
    keywords: Keywords,
    index: u8,
) {
    let words = g.words(item.object);
    let subtypes: Vec<Subtype> = subtypes.iter().map(|&s| words.subtype(s)).collect();
    let animation = crate::Animation {
        power,
        toughness,
        colors: colors.map(|c| words.colors(c)),
        types: TypeSet::NONE,
        subtypes: &subtypes,
        all_creature_types: false,
        replaces_creature_types: false,
        keywords,
        ability: None,
    };
    crate::animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
    g.end_sentence();
    crate::prevention::this_turn(g, item, Target::Object(item.object), index, Shield::All);
}

/// "Put a +1/+1 counter on target [permanent]" (CR 122.1a, 608.2b), the first target still legal on resolution.
/// Bristly Bill, Spine Sower, Bow of Nylea, Yoshimaru, Beloved Companion, Kenrith, the Returned King and
/// Gallia, the Merrymaker print it.
pub fn plus_one_counter_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(object, CounterKind::PlusOne, 1);
    Ok(())
}
