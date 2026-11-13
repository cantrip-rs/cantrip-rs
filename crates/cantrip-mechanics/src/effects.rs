//! One-line effects as functions a `TriggeredDef` or `ActivatedDef` can name, parameterised the way `ward::<N>` is, so
//! a card does not write a wrapper around a single `Game` call. Each acts for the ability's controller or its source.

use cantrip_api::*;

/// Counter the spell carried as the trigger's subject (CR 701.6, 400.7).
/// Chalice of the Void and Presence of the Master use this.
pub fn counter_subject_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        g.counter(spell);
    }
    Ok(())
}

/// "Counter target spell" as a spell's or ability's whole effect (CR 701.6, 608.2b): the spell in the first target slot,
/// countered only while it is still a legal target as this resolves. Cryptic Command, Counterspell, Negate and the
/// Elemental Blasts print it.
pub fn counter_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::counter_target(g, item, 0);
    Ok(())
}

fn you_control_source(q: &dyn Query, e: &ContinuousEffect) -> bool {
    q.is_on_battlefield(e.source) && q.controller(e.source) == e.controller
}

/// A current-state source-control predicate for a custom `Duration::AsLongAs` condition. It cannot reconstruct an
/// interval broken before resolution. Use `Duration::WhileSourceControlled` for the ordinary uninterrupted duration.
pub const YOU_CONTROL_SOURCE: Lasting = Lasting { text: "you control {source}", holds: you_control_source };

/// "Gain control of target [permanent] for as long as you control this creature" (CR 611.2b): the ability's controller gains
/// control of its first target for the uninterrupted `Duration::WhileSourceControlled` interval. Aladdin (artifact)
/// and Thrull Champion (Thrull) print it.
pub fn gain_control_of_target_while_you_control_source(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    let duration = Duration::WhileSourceControlled;
    instruction(g, |g| {
        g.add_effect(item.object, object, EffectKind::Control(item.controller), duration);
    });
    Ok(())
}

/// "[That creature] can't attack or block for as long as you control this [permanent]" (CR 508.1c, 509.1b, 611.2b): both
/// restrictions on `object` use the uninterrupted `Duration::WhileSourceControlled` interval. Extraction Specialist (the
/// creature it returned) and Possession Engine (the creature it took) print it.
pub fn cant_attack_or_block_while_you_control_source(g: &mut dyn Game, item: &StackItem, object: ObjectId) {
    let duration = Duration::WhileSourceControlled;
    instruction(g, |g| {
        g.add_effect(item.object, object, EffectKind::CantAttack, duration);
        g.add_effect(item.object, object, EffectKind::CantBlock, duration);
    });
}

/// "The number of counters on" an object (CR 122.1): counters of every kind, a +1/+1 and a -1/-1 counter both counted while
/// state-based actions have not yet removed the pair (CR 704.5q). Read from the object as it last existed if it has left the
/// battlefield (CR 608.2h). Jadzia Dax, Living Legacy and Savanti Romero, Time's Exile ask it of themselves. A count of one
/// kind is never below zero (`Obj::remove_counters` takes none past what is there), so each is taken as it is.
pub fn counters_on(q: &dyn Query, object: ObjectId) -> u32 {
    let o = q.obj(object);
    CounterKind::ALL.iter().map(|&kind| o.counters(kind) as u32).sum()
}

/// "Look at the top `count` cards of your library, put any number of land cards from among them onto the battlefield tapped, then
/// shuffle" (CR 401.2, 608.2c, 701.24): a library of fewer than `count` shows what it has (CR 609.3); the player sees every card and
/// may take any number of the lands (none included), which enter together, tapped (CR 110.5b, 603.6a); the rest stay in the library,
/// which is then shuffled whether or not a land was taken. Through the Forest Gate (twenty) and Famished Worldsire (its power) print it.
pub fn look_at_top_put_any_lands_onto_the_battlefield_tapped_then_shuffle(g: &mut dyn Game, you: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    let looked = crate::top_of_library(g, you, count);
    g.look_at(you, &looked);
    let rule = crate::select::land_cards(crate::select::ANY_NUMBER);
    let lands = crate::choose::select(g, you, source, Purpose::PutOntoBattlefield, looked, rule)?;
    g.put_all_onto_battlefield_as(&lands.into_iter().map(|land| (land, you, true)).collect::<Vec<_>>())?;
    g.shuffle(you);
    Ok(())
}

/// "This permanent enters with X [kind] counters on it" (CR 614.1c, 122.1), for the X it was cast with; put onto the battlefield
/// without being cast, X is 0 (CR 107.3). `KIND` is `CounterKind::X as usize`. Walking Ballista, Guiding Hydra and the ravenous
/// creatures print it with +1/+1 counters, Chalice of the Void with charge counters and Nissa, Steward of Elements with loyalty
/// counters.
pub fn enters_with_x_counters<const KIND: usize>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[KIND] += entering.x as i32;
    Ok(())
}

/// "This creature enters with a [kind] counter on it" (CR 614.1c, 122.1): `KIND` is `CounterKind::X as usize`. A shield counter
/// (Captain America, Super-Soldier) or a keyword counter (CR 122.1b: Leatherhead, Swamp Stalker's hexproof, Biting-Palm Ninja's
/// menace) are printed this way.
pub fn enters_with_a_counter<const KIND: usize>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[KIND] += 1;
    Ok(())
}

/// "This permanent enters with `N` +1/+1 counters on it" (CR 614.1c, 122.1). Botanical Brawler, Costume Closet (two) and
/// Triskelion (three) print it; one is `enters_with_a_counter`.
pub fn enters_with_plus_one_counters<const N: i32>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += N;
    Ok(())
}

/// "This creature enters with `N` -1/-1 counters on it" (CR 614.1c). Glen Elendra Guardian, Flitterwing Nuisance (one)
/// and Moonshadow (six) print it.
pub fn enters_with_minus_one_counters<const N: i32>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::MinusOne as usize] += N;
    Ok(())
}

/// "Remove a -1/-1 counter from this creature" (CR 122.1): nothing from one that has left the battlefield or has none by now
/// (CR 603.4, 608.2b). Moonshadow, Encumbered Reejerey and Reluctant Dounguard print it as a triggered ability's effect.
pub fn remove_a_minus_one_counter_from_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let source = item.object;
    if crate::targets::permanent(g, source) && g.obj(source).counters(CounterKind::MinusOne) > 0 {
        g.remove_counters(source, CounterKind::MinusOne, 1);
    }
    Ok(())
}

/// "Target creature gets +`P`/+`T` until end of turn" (CR 611.2a), the creature being the ability's first target; nothing
/// if it is no longer a legal target as the spell or ability resolves (CR 608.2b). Bedeck // Bedazzle, Collective
/// Brutality, Dismember, Fervent Champion, Giant Growth, Grim Bauble, Harvester of Misery, Icy Reception, Last Gasp,
/// Locust Spray, Proft, Stab, The Mightstone and Weakstone, Theorix Charm, Umezawa's Jitte and Shambling Ghast print it.
pub fn target_creature_gets<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        crate::pump(g, item.object, creature, P, T);
    }
    Ok(())
}

/// "Target creature gets +`P`/+`T` and gains first strike until end of turn" (CR 611.2a, 702.7): one instruction, the creature
/// being the ability's first target; nothing if it is no longer a legal target as the spell resolves (CR 608.2b). Interjection
/// (+2/+2), Molecular Modifier (+1/+0), Charge the Sanctum's single-target mode (+2/+0) and Riverguard's Reflexes (+2/+2) print it.
pub fn target_creature_gets_and_gains_first_strike<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        instruction(g, |g| {
            crate::pump(g, item.object, creature, P, T);
            gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
        });
    }
    Ok(())
}

/// "Target creature gets +`P`/+`T` and gains haste until end of turn" (CR 611.2a, 702.10): one instruction, the creature
/// being the ability's first target; nothing if it is no longer a legal target as the spell resolves (CR 608.2b). Wild Ride
/// (+3/+0), Fame (+2/+0) and Kavaron Turbodrone (+1/+1) print it.
pub fn target_creature_gets_and_gains_haste<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        instruction(g, |g| {
            crate::pump(g, item.object, creature, P, T);
            crate::haste_until_end_of_turn(g, item.object, creature);
        });
    }
    Ok(())
}

/// "Target creature gets +`P`/+`T` and gains trample until end of turn" (CR 611.2a, 702.19): one instruction, the creature
/// being the ability's first target; nothing if it is no longer a legal target as the spell resolves (CR 608.2b). Blitzball
/// Shot (+3/+3), Fanatical Fever (+3/+0) and Run Amok (+3/+3) print it.
pub fn target_creature_gets_and_gains_trample<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        instruction(g, |g| {
            crate::pump(g, item.object, creature, P, T);
            crate::trample_until_end_of_turn(g, item.object, creature);
        });
    }
    Ok(())
}

/// "One or two target creatures each get +`P`/+`T` until end of turn" (CR 611.2a): every target still legal as the spell
/// resolves (CR 608.2b), pumped as one instruction. Appeal to Eirdu (+2/+1), Opera Love Song's mode (+2/+0) and Spiritcall
/// Enthusiast's Scrollboost (+2/+2) print it.
pub fn targets_get<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = crate::targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            crate::pump(g, item.object, creature, P, T);
        }
    });
    Ok(())
}

/// "One or two target creatures each get +`P`/+`T` and gain `keyword` until end of turn" (CR 611.2a): every target still legal
/// as the ability resolves (CR 608.2b), pumped and given the keyword as one instruction, the two a card would otherwise write
/// as two steps. Heroes' Hangout's Patrol Night (+1/+0 and first strike), Invigorated Rampage's second mode (+2/+0 and trample)
/// and Vic Fontaine, One Night Only (+2/+0 and lifelink) print it.
pub fn targets_get_and_gain(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32, keyword: Keyword) -> Result<(), Suspend> {
    let creatures = crate::targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            crate::pump(g, item.object, creature, power, toughness);
            gain_until_end_of_turn(g, item.object, creature, keyword);
        }
    });
    Ok(())
}

/// "One or two target creatures each gain `keyword` until end of turn" (CR 611.2a): every target still legal as the ability
/// resolves (CR 608.2b), granted as one instruction, as `targets_get` pumps its targets. K't'inga-Class Battle Cruiser (flying), Shuttlecraft Galileo (flying) and Swashbuckler
/// Extraordinaire (double strike) print it.
pub fn targets_gain(g: &mut dyn Game, item: &StackItem, keyword: Keyword) -> Result<(), Suspend> {
    let creatures = crate::targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            gain_until_end_of_turn(g, item.object, creature, keyword);
        }
    });
    Ok(())
}

/// "Destroy all nonland permanents with mana value `limit` or less" (CR 701.8a): every such permanent on the battlefield,
/// destroyed at once. The permanents it destroyed, for what the rest of the text does with them. Culling Ritual (2) and
/// Pest Control (1) print it.
pub fn destroy_nonland_permanents_with_mana_value_at_most(g: &mut dyn Game, limit: u32) -> LeftBattlefield {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| !g.chars(id).is(CardType::Land) && g.mana_value(id) <= limit)
        .collect();
    g.destroy_all(&doomed)
}

/// "Creatures you control gain flying until end of turn if a creature you control has flying. The same is true for
/// first strike, double strike, ..." (CR 611.2a, 613.1f, 702): each ability in `keywords` that at least one creature its
/// controller controls has as the ability resolves is granted to every creature that controller controls until end of
/// turn. The list is the card's (Concerted Effort's, Odric, Lunarch Marshal's hexproof, indestructible, lifelink...);
/// what a card's list holds beyond keywords, such as protection, is that card's own to hand out.
pub fn creatures_you_control_gain_keywords_they_have(g: &mut dyn Game, item: &StackItem, keywords: &[Keyword]) {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let present: Vec<Keyword> = keywords.iter().copied().filter(|&k| creatures.iter().any(|&id| g.chars(id).has(k))).collect();
    instruction(g, |g| {
        for keyword in present {
            for &id in &creatures {
                gain_until_end_of_turn(g, item.object, id, keyword);
            }
        }
    });
}

/// "Destroy all creatures with mana value `min` or more and `max` or less" (CR 701.8a, 202.3): every creature on the
/// battlefield whose mana value is in the range, destroyed at once (CR 101.4). A card in play with {X} in its cost has the
/// X it was cast with (CR 202.3e), which `Query::mana_value` reads. The mana-value-bounded destruction both halves of
/// Austere Command use; the two wrappers below name each half.
pub fn destroy_creatures_with_mana_value_between(g: &mut dyn Game, min: u32, max: u32) -> LeftBattlefield {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && (min..=max).contains(&g.mana_value(id)))
        .collect();
    g.destroy_all(&doomed)
}

/// "Destroy all creatures with mana value `N` or less" (CR 701.8a): `destroy_creatures_with_mana_value_between` from 0.
/// Austere Command's third mode and Ritual of Soot (3) print it.
pub fn destroy_creatures_with_mana_value_at_most<const N: u32>(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    destroy_creatures_with_mana_value_between(g, 0, N);
    Ok(())
}

/// "Destroy all creatures with mana value `N` or greater" (CR 701.8a): `destroy_creatures_with_mana_value_between` with no
/// upper bound. Austere Command's fourth mode prints it.
pub fn destroy_creatures_with_mana_value_at_least<const N: u32>(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    destroy_creatures_with_mana_value_between(g, N, u32::MAX);
    Ok(())
}

/// "Destroy all artifacts with mana value `limit` or less" (CR 701.8a): every such artifact on the battlefield, destroyed at
/// once. The artifacts it destroyed. Brotherhood's End (3), Hammer Mage (X) and Meltdown (X) print it.
pub fn destroy_artifacts_with_mana_value_at_most(g: &mut dyn Game, limit: u32) -> LeftBattlefield {
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
pub fn pump_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) -> Vec<ObjectId> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &id in &creatures {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
    creatures
}

/// "Creatures you control get +`P`/+`T` until end of turn" as a spell's or ability's whole effect: `pump_creatures_you_control`
/// (CR 611.2, 611.2c). Assault Formation, Castle Embereth, Dwarven Provisioner, Fatehold Charm, Haze of Rage, Master of
/// Barbs, Prudent Fateseer, Shatterwing Pegasus, Shefet Dunes, Thorin's Last Stand and Winter, Team Player print it.
pub fn creatures_you_control_get<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_creatures_you_control(g, item, P, T);
    Ok(())
}

/// "Creatures target player controls get +`P`/+`T` until end of turn" (CR 611.2, 611.2c): the creatures the player at target
/// slot `TARGET` controls as it resolves, and no creature that enters later; nothing if that player is no longer a legal target
/// (CR 608.2b). Gnashing of Teeth (-1/-1) and How to Start a Riot (+2/+0) print it.
pub fn creatures_target_player_controls_get<const TARGET: usize, const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = crate::targets::player(g, item, TARGET) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for id in creatures {
            crate::pump(g, item.object, id, P, T);
        }
    });
    Ok(())
}

/// "Creatures you control get +`power`/+`toughness` and gain [a keyword] until end of turn" (CR 611.2, 611.2c): the creatures
/// `item`'s controller controls as it resolves, and no creature that enters later. Silk, Web Weaver (+2/+2) and The Super Hero
/// Civil War (+1/+1) print it with vigilance.
pub fn pump_creatures_you_control_and_gain(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32, keyword: Keyword) {
    instruction(g, |g| {
        for id in pump_creatures_you_control(g, item, power, toughness) {
            gain_until_end_of_turn(g, item.object, id, keyword);
        }
    });
}

/// "Other creatures you control get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): the creatures `item`'s
/// controller controls as it resolves but its source, and no creature that enters later. The creatures, for what the rest
/// of the sentence does to them ("and gain trample"). Restless Prairie and Summon: Esper Maduin print it.
pub fn pump_other_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) -> Vec<ObjectId> {
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &id in &others {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
    others
}

/// "Creatures you control gain [keywords] until end of turn" (CR 611.2, 611.2c, 702): the creatures `item`'s controller controls
/// as it resolves, and no creature that enters later. Bitter Reunion and Ingris Stingerquill print it with haste, Deafening
/// Clarion with lifelink, Spectacular Spider-Man and Invasion of Gobakhan with hexproof and indestructible, and The Wind
/// Crystal with flying and lifelink.
pub fn creatures_you_control_gain(g: &mut dyn Game, item: &StackItem, keywords: &[Keyword]) {
    creatures_you_control_where_gain(g, item, keywords, |_, _| true);
}

/// "Creatures you control [that `wanted`] gain [keywords] until end of turn" (CR 611.2, 611.2c, 702): `creatures_you_control_gain`
/// for the creatures `wanted` accepts as `item` resolves, no creature that enters later (or comes to qualify) gaining anything.
/// Synchronized Charge's "creatures you control with counters on them gain vigilance and trample" prints it.
pub fn creatures_you_control_where_gain(g: &mut dyn Game, item: &StackItem, keywords: &[Keyword], wanted: impl Fn(&dyn Game, ObjectId) -> bool) {
    permanents_you_control_where_gain(g, item, keywords, |g, id| g.chars(id).is(CardType::Creature) && wanted(g, id));
}

/// "Permanents you control [that `wanted`] gain [keywords] until end of turn" (CR 109.2, 611.2c, 702): the permanents of any
/// type `wanted` accepts as `item` resolves, none that enters or comes to qualify later gaining anything. Basri, Tomorrow's
/// Champion's "Cats you control" (a Kindred Artifact with changeling is one) and Synchronized Charge's creatures print it.
pub fn permanents_you_control_where_gain(g: &mut dyn Game, item: &StackItem, keywords: &[Keyword], wanted: impl Fn(&dyn Game, ObjectId) -> bool) {
    let permanents: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| wanted(&*g, id)).collect();
    instruction(g, |g| {
        for id in permanents {
            g.add_effect(item.object, id, EffectKind::AddKeywords(Keywords::of(keywords)), Duration::EndOfTurn);
        }
    });
}

/// "Other creatures you control gain [keywords] until end of turn" (CR 611.2, 611.2c, 702): `creatures_you_control_gain` for the
/// creatures `item`'s controller controls but its source, and no creature that enters later. Blood-Sport Champion's "if it was
/// kicked, other creatures you control gain deathtouch until end of turn" prints it.
pub fn other_creatures_you_control_gain(g: &mut dyn Game, item: &StackItem, keywords: &[Keyword]) {
    let source = item.object;
    creatures_you_control_where_gain(g, item, keywords, move |_, id| id != source);
}

/// "All creatures get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): every creature on the battlefield as
/// `source`'s spell or ability resolves, whoever controls it, and no creature that enters later. Toxic Deluge (-X/-X) and
/// Withering Curse (-2/-2) print it.
pub fn pump_all_creatures(g: &mut dyn Game, source: ObjectId, power: i32, toughness: i32) {
    pump_creatures_where(g, source, |_, _| true, power, toughness);
}

/// "Creatures other than [these] get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): `pump_all_creatures`, with
/// the creatures `other_than` names left out. Zenos yae Galvus's "creatures other than Zenos yae Galvus and the chosen
/// creature get -2/-2" leaves out its source and the creature it chose; `other_creatures_get` is it leaving out only the source.
pub fn pump_creatures_other_than(g: &mut dyn Game, source: ObjectId, other_than: &[ObjectId], power: i32, toughness: i32) {
    pump_creatures_where(g, source, |_, id| !other_than.contains(&id), power, toughness);
}

/// "Other creatures get +`P`/+`T` until end of turn" as a spell's or ability's whole effect: `pump_creatures_other_than`, leaving
/// out only its source (CR 611.2, 611.2c), whoever controls the rest. Harvester of Misery (-2/-2), D'Ghor Bloodletter (-1/-1,
/// "All other creatures") and Shefet Archfiend (-2/-2, "all other creatures") print it.
pub fn other_creatures_get<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_creatures_other_than(g, item.object, &[item.object], P, T);
    Ok(())
}

/// "All nonartifact creatures get -`power`/-`toughness` until end of turn" (CR 611.2c): the creatures on the battlefield at
/// this moment that `counts` accepts, whoever controls them, pumped in one instruction so state triggers wait for all of them
/// (CR 603.3b). Battle of Wolf 359 ("All nonartifact creatures get -X/-X") and Olivia's Wrath ("Each non-Vampire creature
/// gets -X/-X") print it; `pump_all_creatures` and `pump_creatures_other_than` are it with a filter that takes all or all
/// but some.
pub fn pump_creatures_where(g: &mut dyn Game, source: ObjectId, counts: impl Fn(&dyn Game, ObjectId) -> bool, power: i32, toughness: i32) {
    let creatures: Vec<ObjectId> = creatures_on_the_battlefield(g).into_iter().filter(|&id| counts(g, id)).collect();
    instruction(g, |g| {
        for id in creatures {
            crate::pump(g, source, id, power, toughness);
        }
    });
}

/// "Creatures your opponents control get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): the creatures each opponent
/// of `item`'s controller controls as it resolves, and no creature that enters later. Sandman's Quicksand (-2/-2 when its mayhem
/// cost was paid) and Sephiroth, Planet's Heir (-2/-2) print it.
pub fn pump_creatures_your_opponents_control(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) {
    let creatures: Vec<ObjectId> = g
        .state()
        .opponents(item.controller)
        .flat_map(|opponent| g.permanents_of(opponent))
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for id in creatures {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
}

/// "Creatures you control gain trample and get +P/+T until end of turn" (CR 611.2c): `pump_creatures_you_control` and trample for
/// each of them, the creatures being those on the battlefield as it resolves. The creatures, for what the rest of the sentence does
/// to them. Craterclaw Colossus (+X/+0, X the artifacts you control), Craterhoof Behemoth (+X/+X, X the creatures) and Balmor,
/// Battlemage Captain (+1/+0) print it.
pub fn pump_and_trample_creatures_you_control(g: &mut dyn Game, item: &StackItem, power: i32, toughness: i32) -> Vec<ObjectId> {
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
pub fn counters_on_creatures_you_control(g: &mut dyn Game, player: PlayerId, kind: CounterKind, n: i32) -> Vec<ObjectId> {
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    counters_on_each(g, &creatures, kind, n);
    creatures
}

/// "Put a +1/+1 counter on each creature you control" as a spell's or ability's whole effect: on the creatures its controller
/// controls as it resolves (`counters_on_creatures_you_control`, CR 122.1, 608.2). Abandoned Air Temple, Cosmogrand Zenith (a mode),
/// Denzilore Fatehold, Gavony Township, Origin of the Avengers (chapter III) and Wilderland Scrounger print it.
pub fn plus_one_counter_on_each_creature_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_creatures_you_control(g, item.controller, CounterKind::PlusOne, 1);
    Ok(())
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
pub fn counters_on_each(g: &mut dyn Game, objects: &[ObjectId], kind: CounterKind, n: i32) {
    let puts: Vec<(ObjectId, CounterKind, i32)> = objects.iter().map(|&id| (id, kind, n)).collect();
    g.add_counters_together(&puts);
}

/// "Put `n` `kind` counters on each [subtype] you control": every permanent the ability's controller controls as it resolves
/// that has the subtype (CR 608.2c, 205.3), the source among them, the subtype read through the words of the source's text
/// (CR 612.2); all of them at once (CR 608.2, 101.4: one event, CR 603.2c). The permanents. Lyra, Archangel of Dawn
/// (Angels) and Ajani, Nacatl Avenger (Cats) print it, with one +1/+1 counter: `plus_one_counter_on_each_of_subtype_you_control`.
pub fn counters_on_each_of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, kind: CounterKind, n: i32) -> Vec<ObjectId> {
    of_subtype_you_control(g, item, subtype, false, false, kind, n)
}

/// "Other [subtype]s you control gain [keywords] until end of turn" (CR 611.2, 611.2c, 702): every permanent with the subtype that `item`'s
/// controller controls as it resolves but its source, the subtype read through the words of the source's text (CR 612.2), and none that
/// enters later. `SUBTYPE` is a `Subtype` as `u16` (`lands::sub`), `KEYWORDS` a `Keywords` as `u128` (`Keywords::of(&[Keyword::Flying]).0`).
/// Cosmic Spider-Man (Spiders, five keywords) and Vulture, Scheming Scavenger (Villains, flying) print it.
pub fn other_of_subtype_you_control_gain_until_end_of_turn<const SUBTYPE: u16, const KEYWORDS: u128>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let word = g.words(item.object).subtype(crate::lands::printed(SUBTYPE));
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).has_subtype(word)).collect();
    instruction(g, |g| {
        for id in others {
            g.add_effect(item.object, id, EffectKind::AddKeywords(Keywords(KEYWORDS)), Duration::EndOfTurn);
        }
    });
    Ok(())
}

/// "Put `n` `kind` counters on each [subtype] creature you control" (CR 122.1, 205.3, 608.2): `counters_on_each_of_subtype_you_control`
/// for the permanents of the subtype that are creatures as it resolves, so a noncreature permanent with the subtype gets none. Mindspring
/// Merfolk (Merfolk, +1/+1) and Avenger of Zendikar (Plant, +1/+1) print it.
pub fn counters_on_each_creature_of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, kind: CounterKind, n: i32) -> Vec<ObjectId> {
    of_subtype_you_control(g, item, subtype, false, true, kind, n)
}

/// "Put `n` `kind` counters on each other [subtype] you control": `counters_on_each_of_subtype_you_control` without the
/// source. Thalia's Lieutenant (Humans), Rescue Retriever (Soldiers), Camellia, the Seedmiser (Squirrels), Agent Phil
/// Coulson and Doctor Spectrum (Heroes) and Vampire Socialite (Vampires) print it, with one +1/+1 counter:
/// `plus_one_counter_on_each_other_of_subtype_you_control`.
pub fn counters_on_each_other_of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, kind: CounterKind, n: i32) -> Vec<ObjectId> {
    of_subtype_you_control(g, item, subtype, true, false, kind, n)
}

fn of_subtype_you_control(g: &mut dyn Game, item: &StackItem, subtype: Subtype, other: bool, creature: bool, kind: CounterKind, n: i32) -> Vec<ObjectId> {
    let word = g.words(item.object).subtype(subtype);
    let objects: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| !(other && id == item.object) && (!creature || g.chars(id).is(CardType::Creature)) && g.chars(id).has_subtype(word))
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

/// "[Subtype] you control get +`power`/+`toughness` until end of turn" (CR 611.2, 611.2c): the permanents of that subtype the
/// ability's controller controls as it resolves (CR 205.3), the source among them, and none that enters later, the subtype
/// read through the words of the source's text (CR 612.2). The permanents. Ran and Shaw (Dragons, +2/+0) and Summon: Esper
/// Ramuh (Wizards, +1/+0) print it.
pub fn subtype_you_control_get(g: &mut dyn Game, item: &StackItem, subtype: Subtype, power: i32, toughness: i32) -> Vec<ObjectId> {
    subtypes_you_control_get(g, item, [subtype, subtype], power, toughness)
}

/// "[Subtype] and [subtype] you control get +`power`/+`toughness` until end of turn" (CR 109.2, 611.2c): the permanents, of any
/// card type, that the ability's controller controls as it resolves and that have either subtype (a Klingon Warrior once),
/// the source among them, and none that enters later. A noncreature one is pumped too and keeps it if it is animated later
/// that turn. Each subtype is read through the words of the source's text
/// (CR 612.2). The permanents, for what the rest of the sentence does to them ("and gain trample"). Kor, Cause of Vengeance
/// ("Klingons and Warriors", +2/+2) and Gnarling Targ ("Klingons and Warriors", +2/+0) print it; `subtype_you_control_get` is it
/// with one subtype.
pub fn subtypes_you_control_get(g: &mut dyn Game, item: &StackItem, subtypes: [Subtype; 2], power: i32, toughness: i32) -> Vec<ObjectId> {
    let words = subtypes.map(|subtype| g.words(item.object).subtype(subtype));
    let objects: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| words.iter().any(|&word| g.chars(id).has_subtype(word))).collect();
    instruction(g, |g| {
        for &id in &objects {
            crate::pump(g, item.object, id, power, toughness);
        }
    });
    objects
}

/// "Put a +1/+1 counter on each other [subtype] you control" as an ability's whole effect: `counters_on_each_other_of_subtype_you_control`.
/// Thalia's Lieutenant, Rescue Retriever, Camellia, the Seedmiser, Agent Phil Coulson, Doctor Spectrum and Vampire Socialite.
pub fn plus_one_counter_on_each_other_of_subtype_you_control<const SUBTYPE: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_each_other_of_subtype_you_control(g, item, crate::lands::printed(SUBTYPE), CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put a +1/+1 counter on each other creature you control" as an ability's whole effect (CR 122.1, 101.4): the creatures its controller controls
/// as it resolves but its source, together. Monica Rambeau and Web-Warriors print it.
pub fn plus_one_counter_on_each_other_creature_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    counters_on_each(g, &others, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put two +1/+1 counters on target [permanent]" (CR 122.1a, 608.2b), the first target still legal on resolution. The whole
/// effect of the sacrifice ability of Elvenking's Halls, Goblin-town, Iron Hills, Lake-town and Mirkwood (`lands::sacrifice_for_two_counters`),
/// and of Room of Refuge, Trash the Town and Silverquill Charm: with none chosen (an "up to one" slot) or none legal nothing is put on.
pub fn two_plus_one_counters_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(object, CounterKind::PlusOne, 2);
    Ok(())
}

/// "Distribute N +1/+1 counters among [one or more target creatures]" (CR 601.2d, 122.1): each target its share, fixed as
/// the spell or ability was put on the stack, a target illegal as it resolves left out with its share (CR 608.2b; the
/// rulings of Jade Seedstones and The Earth Crystal). Jade Seedstones, Elusive Otter's Grove's Bounty and The Earth
/// Crystal print it.
pub fn distribute_plus_one_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::counters_divided(g, item, CounterKind::PlusOne);
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

/// "This Vehicle becomes an artifact creature. Put `N` +1/+1 counters on it" (CR 301.7, 611.2a, 122.1): with no duration said it
/// stays an artifact creature for as long as it is on the battlefield, still a Vehicle, its printed power and toughness and the
/// counters making it. Done to the permanent whose ability this is, so a Vehicle that has left the battlefield by the time the
/// ability resolves is a new object and gets nothing (CR 400.7). The exhaust ability of Invasion Submersible (three counters) and
/// Rangers' Refueler (one) is it.
pub fn this_vehicle_becomes_artifact_creature_with_plus_one_counters<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::Indefinite);
    g.add_counters(item.object, CounterKind::PlusOne, N);
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

/// "Put three +1/+1 counters on [this permanent]" (CR 122.1, 201.5): the ability's source, not a new object once it has left the
/// battlefield (CR 400.7). The whole effect of an ability of Aerial Doombot (power-up) and Guardian of the Halls.
pub fn three_plus_one_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 3);
    Ok(())
}

/// "Put that many +1/+1 counters on [this permanent]" (CR 122.1, 603.2c) after a "whenever you ... one or more" trigger
/// with `.once_per_batch()`: one counter for each object of the batch the trigger saw, counted even for one that has since
/// left. The ability's source, not a new object once it has left the battlefield (CR 400.7). The whole effect of Marauding
/// Mako's and Scrounging Skyray's triggers.
pub fn that_many_plus_one_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.event_objects(item).len() as i32;
    g.add_counters(item.object, CounterKind::PlusOne, many);
    Ok(())
}

/// "[This creature] gets +`P`/+`T` until end of turn" (CR 611.2a, 613.4c): a continuous effect from the ability's source, which
/// does nothing once the source has left the battlefield (CR 400.7). Each resolution adds its own, and they stack. The whole
/// effect of an ability of Attercop, Basking Rootwalla, Blazing Rootwalla, Boughside Wanderers, Burnout Bashtronaut, Desolation
/// Prowler, Diviner of Victory, Flameblade Adept, Hydro-Man, Fluid Felon, Knight of Dusk's Shadow, Mausoleum Wanderer,
/// Nighthowl Pursuer, Ravine Raider, Remnant Elemental, Skilled Battlecarver, Slickshot Show-Off, Undulating Witness, Veyran, Voice of Duality
/// and Woodland Weavemaster.
pub fn this_gets<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::pump(g, item.object, item.object, P, T);
    Ok(())
}

/// "[This creature] gets +`P`/+`T` and gains menace until end of turn" (CR 611.2a, 613.4c, 702.111): one instruction ("gets +1/+0 and
/// gains menace") of two continuous effects from the ability's source, both of which do nothing once it has left the battlefield
/// (CR 400.7). Flame-Chain Mauler and Inner Demons Gangsters (+1/+0) print it.
pub fn this_gets_and_gains_menace<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        crate::pump(g, item.object, item.object, P, T);
        if g.is_on_battlefield(item.object) {
            gain_until_end_of_turn(g, item.object, item.object, Keyword::Menace);
        }
        Ok(())
    })
}

/// "[This creature] gets +`P`/+`T` and gains flying until end of turn" (CR 611.2a, 613.4c, 702.9a): one instruction of two
/// continuous effects from the ability's source, both of which do nothing once it has left the battlefield (CR 400.7).
/// Brightfield Glider (+1/+2, when it attacks while saddled) and Kithkeeper (+3/+0) print it.
pub fn this_gets_and_gains_flying<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        crate::pump(g, item.object, item.object, P, T);
        if g.is_on_battlefield(item.object) {
            gain_until_end_of_turn(g, item.object, item.object, Keyword::Flying);
        }
        Ok(())
    })
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

/// "Destroy all [card type]" as a spell's whole effect, `TYPE` a `CardType` as `u8`: every permanent of that type on the
/// battlefield as the spell resolves, whoever controls it, destroyed at once (CR 701.8a, 608.2c, 101.4); regeneration and
/// indestructible are the verb's business. Armageddon (lands) and Tranquility (enchantments) print it.
pub fn destroy_all_of_type<const TYPE: u8>(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let doomed: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).types.0 & (1 << TYPE) != 0).collect();
    g.destroy_all(&doomed);
    Ok(())
}

/// "Destroy all [subtype]" as a spell's whole effect, `SUBTYPE` a `Subtype` as `u16` (`lands::sub`): every permanent on the
/// battlefield with the subtype as the spell resolves, whoever controls it, destroyed at once (CR 701.8a, 608.2c, 101.4), the
/// subtype read through the words of the spell's text (CR 612.2), so a text change ("Plains" to "Islands") changes what it
/// destroys. Flashfires (Plains), Tsunami and Boil (Islands) and Acid Rain (Forests) print it.
pub fn destroy_all_of_subtype<const SUBTYPE: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let word = g.words(item.object).subtype(crate::lands::printed(SUBTYPE));
    let doomed: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).has_subtype(word)).collect();
    g.destroy_all(&doomed);
    Ok(())
}

/// "Target creature can't block this turn" (CR 509.1b, 611.2a): the resolving object's first target, if it is still legal (CR
/// 608.2b), can't block until end of turn - an effect on the creature, not an ability it gains. Zirda, the Dawnwaker, Aragorn,
/// King of Gondor (up to one target) and Hawkeye, Master Marksman print it.
pub fn first_target_cant_block_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::CantBlock, Duration::EndOfTurn);
    }
    Ok(())
}

/// "Draw N cards" (CR 121.1). Chromatic Star and Mind Stone print "draw a card"; Coveted Jewel and Bankrupt in Blood draw
/// three.
pub fn draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    g.draw(item.controller, n);
    Ok(())
}

/// "You gain LIFE life and draw CARDS card(s)" (CR 119.3, 121.1): the controller gains the life, then draws. Candy Trail and
/// Chevill, Bane of Monsters print it with 3 life and a card, Inspiring Overseer with 1 life and a card.
pub fn gain_life_then_draw<const LIFE: i32, const CARDS: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, LIFE)?;
    draw(g, item, CARDS)
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
    g.draw(item.controller, n);
    put_cards_from_hand_on_top(g, item.controller, item.object, 2)
}

/// "[Player] chooses N cards from their hand and puts them on top of their library in any order" (CR 608.2d, 401.4): the cards are
/// chosen together (as many as the hand holds, up to `n`) and put back one at a time in the order chosen, so the last chosen ends
/// on top. Brainstorm, Brainsurge and Jace, the Mind Sculptor's 0 ability (two, through `draw_then_put_two_back`) and Stunted
/// Growth (three, for the player it targets) print it.
pub fn put_cards_from_hand_on_top(g: &mut dyn Game, player: PlayerId, source: ObjectId, n: u32) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let cards = crate::choose::select(g, player, source, Purpose::PutOnTop, hand, crate::select::cards(crate::select::as_many_as_can(n)))?;
    // The last card chosen ends up on top.
    let order: Vec<ObjectId> = cards.iter().rev().copied().collect();
    g.put_into_library(&order, LibraryPlace::Top);
    Ok(())
}

/// "Draw a card, then you may put a land card from your hand onto the battlefield tapped" (CR 121.1, 608.2c, 608.2d,
/// 614.1c): the draw ends the sentence, so the card just drawn is in the hand to choose from and may be the one put; any
/// land card of the hand may be chosen, or none. Putting a land onto the battlefield is not playing it, so it uses no land
/// play (CR 305.2). Jefferies Tube and H.E.R.B.I.E. Scout Unit print it.
pub fn draw_a_card_then_may_put_a_land_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    crate::hand::may_put_onto_battlefield_tapped(g, item.controller, item.object, |g, c| g.chars(c).is(CardType::Land))?;
    Ok(())
}

/// "Target player draws N cards" (CR 121.1): the player the item targeted in `slot`, if the target is still legal (CR 608.2b).
/// Ancestral Recall (three), Flame of Anor and Ashling's Command (two) print it.
pub fn target_player_draws(g: &mut dyn Game, item: &StackItem, slot: usize, n: u32) {
    if let Some(player) = crate::targets::player(g, item, slot) {
        g.draw(player, n);
    }
}

/// "Target player draws `CARDS` cards and loses `LIFE` life" (CR 121.1, 119.3): both happen to the player the item targeted
/// in its first slot, if it is still a legal target (CR 608.2b), and the life is lost even when the library was empty (CR 121.4).
/// Decorum Dissertation, Reverent Howl and Sign in Blood print it, of two cards and 2 life.
pub fn target_player_draws_and_loses_life<const CARDS: u32, const LIFE: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        g.draw(player, CARDS);
        g.lose_life(player, LIFE);
    }
    Ok(())
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
    loot_discarding(g, item.controller, item.object)?;
    Ok(())
}

/// `loot` for an ability that goes on to ask about the card: the cards the discard actually discarded, wherever they went (a
/// graveyard, or exile by a madness replacement), none for an empty hand. Ninja's Blades loses the damaged player life equal to its mana value.
pub fn loot_discarding(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<Vec<ObjectId>, Suspend> {
    loot_discarding_or_may(g, player, source, false)
}

/// `loot_discarding` for "draw a card, then you may discard a card" when `may` (CR 608.2d): declining is the empty answer to the
/// up-to-one choice.
fn loot_discarding_or_may(g: &mut dyn Game, player: PlayerId, source: ObjectId, may: bool) -> Result<Vec<ObjectId>, Suspend> {
    g.draw(player, 1);
    // A draw replacement may change state without recording an event (CR 603.8, 608.2c).
    g.end_sentence();
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let chosen = crate::choose::cards(g, player, source, Purpose::Discard, hand, 1, may)?;
    Ok(g.discard_together(&chosen))
}

/// "Draw a card, then discard a card. When you discard a nonland card this way, ..." (CR 603.12): the controller draws, then
/// discards a card they choose (CR 701.9, nothing asked of an empty hand), and `ability` - the card's reflexive ability - is put
/// on the stack only when a card discarded this way is a nonland card, judged by what it is where it went (CR 701.9c, an
/// unrevealed hidden-zone discard is undefined). Teo, Spirited Glider and Seven of Nine, Tertiary Adjunct print it.
pub fn loot_then_reflexive(g: &mut dyn Game, item: &StackItem, ability: AbilityRef) -> Result<(), Suspend> {
    loot_then_reflexive_discarding(g, item, ability, false)
}

/// `loot_then_reflexive` for "draw a card, then you may discard a card" (CR 608.2d): declining is the empty answer to the
/// up-to-one choice, and an empty hand offers none, so nothing triggers either way. Glacial Dragonhunt prints it.
pub fn loot_then_maybe_reflexive(g: &mut dyn Game, item: &StackItem, ability: AbilityRef) -> Result<(), Suspend> {
    loot_then_reflexive_discarding(g, item, ability, true)
}

fn loot_then_reflexive_discarding(g: &mut dyn Game, item: &StackItem, ability: AbilityRef, may: bool) -> Result<(), Suspend> {
    let you = item.controller;
    let discarded = loot_discarding_or_may(g, you, item.object, may)?;
    if discarded.iter().any(|&card| crate::hand::discarded_nonland_card(&*g, card)) {
        g.reflexive_trigger(item.object, you, ability, Target::None);
    }
    Ok(())
}

/// "[Player] discards any number of cards, then draws that many cards" (CR 701.9, 121.1): `player` chooses any number of cards of
/// their hand and discards them as one event, then draws as many as were discarded; nothing is asked of an empty hand and nothing
/// is drawn for no cards. Leave // Chance discards and draws for its caster, Steal the Show for its target player.
pub fn discard_any_number_then_draw(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    let hand: Vec<ObjectId> = g.state().zone(Zone::hand(player)).to_vec();
    let cards = crate::choose::any_number_of(g, player, source, Purpose::Discard, "", hand.clone(), hand)?;
    if cards.is_empty() {
        return Ok(());
    }
    g.discard_together(&cards);
    g.draw(player, cards.len() as u32);
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

/// "You and target opponent each draw `n` cards" (CR 121.1, 121.2c): the ability's controller and the target player in slot `slot`
/// draw, in turn order from the active player, who is not always the controller (an ability can be activated on an opponent's
/// turn). With the target illegal the ability does not resolve (CR 608.2b), so the controller draws nothing either. Loran of
/// the Third Path and Joined Researchers' Secret Rendezvous print it.
pub fn you_and_target_player_each_draw(g: &mut dyn Game, item: &StackItem, slot: usize, n: u32) {
    let Some(target) = crate::targets::player(g, item, slot) else { return };
    let players: Vec<PlayerId> = g.state().apnap().filter(|&p| p == item.controller || p == target).collect();
    for player in players {
        g.draw(player, n);
    }
}

/// "Draw two cards, then discard a card" (CR 121.1, 701.9): the controller draws two, then chooses a card of their hand, the
/// two drawn included, and discards it. Overlord of the Floodpits, Roiling Dragonstorm and Wistfulness print it.
pub fn draw_two_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    crate::discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "Draw two cards, then discard two cards" (CR 121.1, 701.9): the controller draws two, then chooses two cards of their hand,
/// the two drawn included, and discards them. Faithless Looting, Jadzi, Steward of Fate and Spellbook Seeker's Careful Study
/// print it.
pub fn draw_two_then_discard_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    crate::discard_chosen(g, item.controller, item.object, 2)?;
    Ok(())
}

/// "Draw three cards, then discard a card" (CR 121.1, 701.9): the controller draws three, then chooses a card of their hand, the
/// three drawn included, and discards it. Coil and Catch (Marang River Regent's Omen) and Unexpected Assistance print it.
pub fn draw_three_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
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

/// "You may put a card from your hand on the bottom of your library. If you do, draw a card" (CR 401.4, 121.1): the card is
/// chosen as the effect resolves, so an empty hand is asked nothing; the draw follows a card actually put there (CR 608.2c).
/// Volcanic Spite and Manhole Missile print it after their damage.
pub fn may_put_a_card_from_hand_on_the_bottom_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let chosen = crate::choose::select(g, item.controller, item.object, Purpose::PutOnBottom, hand, crate::select::cards(crate::select::up_to(1)))?;
    if !g.put_into_library(&chosen, LibraryPlace::Bottom).is_empty() {
        g.draw(item.controller, 1);
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

/// "You may discard a [kind of] card" (CR 701.9): the player chooses, as the effect resolves and not as a cost (CR 608.2d), up to
/// what `rule` allows of the cards of their hand, or none (asked whenever the hand has a card). Returns whether a
/// card was discarded, for the reflexive "when you do" (CR 603.12) that follows: a card chosen that its owner can't be made to
/// discard stays (CR 101.2), and then nothing was done. Dokuchi Silencer (a creature card) and Hypothesizzle (a nonland card) print it.
pub fn may_discard(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection) -> Result<bool, Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let chosen = crate::choose::select(g, player, source, Purpose::Discard, hand, rule)?;
    if chosen.is_empty() {
        return Ok(false);
    }
    Ok(!g.discard_together(&chosen).is_empty())
}

/// "Sacrifice any number of lands, then search your library for up to that many land cards, put them onto the battlefield
/// tapped, then shuffle" (CR 701.21a, 608.2c-d, 701.23b). The number found is the number actually sacrificed; the search and
/// shuffle still happen when the player sacrifices none. Scapeshift and Genesis Device print it.
pub fn sacrifice_any_number_of_lands_for_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let lands: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&p| g.chars(p).is(CardType::Land)).collect();
    let chosen = crate::choose::select(g, you, item.object, Purpose::Sacrifice, lands, crate::select::land_cards(crate::select::ANY_NUMBER))?;
    // "That many" is the lands sacrificed, a merged permanent once (CR 730.2), not the cards they became.
    let sacrificed = g.sacrifice(you, &chosen).count();
    g.end_sentence();
    crate::library::find_to_battlefield(g, you, item.object, crate::select::land_cards(crate::select::up_to(sacrificed)), true)?;
    Ok(())
}

/// "You may sacrifice a [quality]" (CR 701.21a): the player chooses, as the effect resolves and not as a cost (CR 608.2d), up to
/// what `rule` allows of their permanents, or none. The engine offers only what the player controls and can sacrifice (CR 101.2),
/// and nothing is asked when none qualifies. Returns what was actually sacrificed, so "if you do" and "when you do" (CR 608.2c,
/// 603.12) are a non-empty check. Tenured Tethermage (a land), Way of the Deathbringer and Grist, the Hunger Tide (a creature), Winter,
/// Tormented Loner (a creature or planeswalker), Entrust the Spark (a planeswalker) and Witherbloom Charm (a permanent) print it.
pub fn may_sacrifice(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection) -> Result<LeftBattlefield, Suspend> {
    let permanents = g.permanents_of(player);
    let chosen = crate::choose::select(g, player, source, Purpose::Sacrifice, permanents, rule)?;
    if chosen.is_empty() {
        return Ok(LeftBattlefield::default());
    }
    Ok(g.sacrifice(player, &chosen))
}

/// "You may sacrifice an artifact or discard [a kind of] card" (CR 608.2c-d, 701.9, 701.21): the player chooses as the effect
/// resolves which of the two to do, or neither; the discard is one card `discard` allows of their hand. An option with nothing
/// to do it with (no artifact, no card of the kind in hand) is not offered, and nothing is asked with neither possible.
/// Returns whether the sacrifice or the discard actually happened, for the "if you do" or reflexive "when you do" (CR 608.2c,
/// 603.12) that follows. The options are the card's `labels`, in the order "Sacrifice an artifact", "Discard ...", "Neither".
/// Chandra, Spark Hunter and Vision of Love (a card) and Bullseye, Death Dealer (a nonland card) print it.
pub fn may_sacrifice_an_artifact_or_discard(g: &mut dyn Game, player: PlayerId, source: ObjectId, discard: Selection) -> Result<bool, Suspend> {
    const SACRIFICE: u8 = 0;
    const DISCARD: u8 = 1;
    const NEITHER: u8 = 2;
    // An artifact that can't be sacrificed is nothing to do it with (CR 101.2: the option can't be taken).
    let artifacts: Vec<ObjectId> =
        g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Artifact) && g.can_sacrifice(player, id)).collect();
    let rule = discard.taking(crate::select::exactly(1));
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let mut options = Vec::new();
    if !artifacts.is_empty() {
        options.push(SACRIFICE);
    }
    if cantrip_api::select::first_answer(&*g, &rule, player, source, &hand, true).is_some_and(|cards| !cards.is_empty()) {
        options.push(DISCARD);
    }
    if options.is_empty() {
        return Ok(false);
    }
    options.push(NEITHER);
    let card = g.obj(source).card;
    let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
    Ok(match options[crate::choose::one_of(g, player, source, labels)? as usize] {
        SACRIFICE => {
            let chosen = crate::choose::cards(g, player, source, Purpose::Sacrifice, artifacts, 1, false)?;
            !g.sacrifice(player, &chosen).is_empty()
        }
        DISCARD => {
            let chosen = crate::choose::select(g, player, source, Purpose::Discard, hand, rule)?;
            !g.discard_together(&chosen).is_empty()
        }
        _ => false,
    })
}

/// "You may sacrifice an artifact or discard a card. If you do, draw `n` cards" (CR 608.2c-d, 701.9, 701.21):
/// `may_sacrifice_an_artifact_or_discard` for any card, then the cards are drawn only if it happened. Chandra, Spark Hunter
/// (one card) and Vision of Love (two) print it.
pub fn may_sacrifice_an_artifact_or_discard_to_draw(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    if may_sacrifice_an_artifact_or_discard(g, item.controller, item.object, crate::select::CARD)? {
        g.draw(item.controller, n);
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

/// "You may discard your hand" (CR 608.2c-d, 701.9): its controller is asked as the ability resolves, and their whole hand
/// goes at once if they accept. An empty hand may be discarded too, and that counts as having done it.
///
/// Says how many cards were discarded, for a "draw a card for each card discarded this way" (CR 121.1) - which is settled
/// before any card is drawn - or `None` when the player declined. Narset, Jeskai Waymaster, Vaultguard Trooper, Phyrexian
/// Dragon Engine and U.S.S. Enterprise-C, Time-Tossed print it.
pub fn may_discard_hand(g: &mut dyn Game, item: &StackItem, question: &'static Question) -> Result<Option<u32>, Suspend> {
    let player = item.controller;
    if !ask_may(g, player, item.object, Purpose::Ask(question))? {
        return Ok(None);
    }
    Ok(Some(g.discard_hand(player).len() as u32))
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

const WHEEL: Question = Question::new(
    "DiscardHandToDrawSeven",
    "discard your hand and draw seven cards?",
    "You may discard even an empty hand",
    "Discard hand, draw seven",
    "Keep hand",
)
.no_default();

/// "Each player may discard their hand and draw seven cards." Choices are made in APNAP order before the simultaneous
/// discards (CR 101.4, 701.9a); the consenting players then draw in APNAP order (CR 121.2c). A player who can't draw seven
/// cards is not offered the choice (CR 121.2b). Arc of Fortune and Raphael's Technique print it.
pub fn each_player_may_discard_hand_and_draw_seven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let mut willing = Vec::new();
    for player in players {
        if !g.cant_draw(player, 7) && ask_may(g, player, item.object, Purpose::Ask(&WHEEL))? {
            willing.push(player);
        }
    }
    players_discard_hands_then_draw_seven(g, &willing);
    Ok(())
}

/// "Target player gains N life" (CR 119.3, 608.2b): the first targeted player gains N life if still legal. Healing Salve,
/// Kenrith, the Returned King and Titan of Industry print it.
pub fn target_player_gains<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    target_player_gains_life(g, item, N)
}

/// "Target player gains X life": Alabaster Potion and Stream of Life (also prepared by Infirmary Healer).
pub fn target_player_gains_x_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        g.gain_life(player, item.x.min(i32::MAX as u32) as i32);
    }
    Ok(())
}

/// "When this creature dies, you gain N life" (CR 700.4, 603.10a, 119.3).
/// Onulet and Haywire Mite use this death trigger.
pub const fn dies_gain_life<const N: i32>(text: &'static str) -> AbilityDef {
    triggered(crate::triggers::THIS_DIES, gain_life_amount::<N>, text)
}

fn gain_life_amount<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, N)
}

/// "You gain N life" (CR 119.3). Zuran Orb and Inventor's Fair print it (two and one).
pub fn gain_life(g: &mut dyn Game, item: &StackItem, n: i32) -> Result<(), Suspend> {
    g.gain_life(item.controller, n);
    Ok(())
}

/// "You gain life equal to the number of differently named lands you control" (CR 201.2a, 608.2c): counted as the
/// ability resolves, two lands with the same name counting once. Species 8472 and Christine Chapel, Combat Medic print
/// it, both on "when this creature enters".
pub fn gain_life_for_differently_named_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = crate::lands::differently_named_lands(g, item.controller) as i32;
    gain_life(g, item, n)
}

/// "You gain life equal to its toughness" (CR 119.3, 608.2h) as a triggered ability's callback: the toughness of the trigger's
/// subject (`StackItem::subject`, a permanent that entered or a creature that died) as the layers make it, or as it last
/// existed on the battlefield if it has left. A permanent that is not a creature as the ability resolves has no toughness (CR 208.3) and
/// gains nothing; neither does a toughness not above zero. South Wind Avatar ("that creature died") and Righteous Valkyrie
/// ("that creature entered") print it.
pub fn gain_life_equal_to_subject_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let chars = g.chars(creature);
    if !chars.is(CardType::Creature) {
        return Ok(());
    }
    gain_life(g, item, i32::from(chars.toughness.unwrap_or_default()).max(0))
}

/// "You gain 3 life" as an ability's callback: the Food ability "{2}, {T}, Sacrifice this permanent: You gain 3 life." (CR 111.10b)
/// of the Food token, Ygra, Eater of All's creatures, Nutrient Block and Spicy Oatmeal Pizza.
pub fn gain_three_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, 3)
}

/// "You gain that much life" as a triggered ability (CR 119.3): the amount the trigger carries in `data[0]`. Bloodthirsty
/// Conqueror (life lost) prints it. Damage uses gain_matched_damage_life instead.
pub fn gain_that_much_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, item.data[0] as i32)
}

/// "{cost}: Target creature gets +`P`/+`T` for as long as this artifact remains tapped" (CR 611.2a, 608.2b): the creature the
/// resolving ability targeted in slot 0, if it is still a legal target. Ashnod's Battle Gear (+2/-2) and Tawnos's Weaponry
/// (+1/+1) print it.
pub fn target_creature_gets_while_source_tapped<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| g.add_effect(item.object, creature, EffectKind::Pump(P, T), Duration::WhileSourceTapped));
    Ok(())
}

/// Gain the bounded quantity frozen by `.damage_amount()` from the matched
/// damage events. This has no legacy `data[0]` fallback.
pub fn gain_matched_damage_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, item.matched_damage_amount() as i32);
    Ok(())
}

/// "You gain 1 life" as a triggered or activated ability. Koth of the Homestead and Medic's Kitesail use this callback.
pub fn gain_one_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, 1)
}

/// "`player` loses half their life, rounded up" (CR 107.1a, 119.3): their life total as this is done. A life total of zero or
/// less has no half to lose. Unstoppable Slasher and Dark Leo & Shredder print it for the player they damaged.
pub fn lose_half_their_life(g: &mut dyn Game, player: PlayerId) {
    let life = g.state().player(player).life;
    g.lose_life(player, (life.max(0) + 1) / 2);
}

/// "They lose half their life, rounded up" of a trigger whose subject is the damaged player (see `lose_half_their_life`):
/// Unstoppable Slasher's whole ability.
pub fn subject_player_loses_half_their_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(player) = item.subject {
        lose_half_their_life(g, player);
    }
    Ok(())
}

/// "Each opponent loses life equal to the life that player lost this turn" (CR 119.3, 608.2h): Archfiend of Despair and Gul Dukat, Colonial Prefect read all losses before losing them together (CR 101.4); returns the actual loss.
pub fn each_opponent_loses_life_lost_this_turn(g: &mut dyn Game, player: PlayerId) -> i32 {
    let losses: Vec<_> = g.state().opponents(player).map(|opponent| (opponent, g.history().life_lost(opponent) as i32)).collect();
    g.lose_life_together(&losses)
}

/// "Each opponent loses `N` life and you gain `N` life" (CR 119.3): every opponent still in the game loses it together, and the
/// ability's controller gains `N`. Gollum, Riddle Master, Unholy Annex and Sneering Shadewriter print it of 2, and
/// `each_opponent_loses_1_life_you_gain_1` is of 1.
pub fn each_opponent_loses_life_you_gain_life<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, N);
    g.gain_life(item.controller, N);
    Ok(())
}

/// "Each opponent loses 1 life and you gain 1 life" (CR 119.3): `each_opponent_loses_life_you_gain_life` of 1. Bontus's
/// Monument, Doom Reigns Supreme, Grave Venerations, Raven of Fell Omens, Thunder of Unity, Tithing Blade, Witherbloom
/// Apprentice and Dream Beavers print it.
pub fn each_opponent_loses_1_life_you_gain_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_loses_life_you_gain_life::<1>(g, item)
}

/// "Each opponent loses 1 life" (CR 119.3): every opponent still in the game loses it together. Lunar Convocation, Starscape
/// Cleric and The Meathook Massacre print it.
pub fn each_opponent_loses_1_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 1);
    Ok(())
}

/// "Each opponent loses 2 life" (CR 119.3): every opponent still in the game loses it together. Engine Rat, Leech Collector's
/// Bloodletting and The Coming of Galactus's chapters II and III print it.
pub fn each_opponent_loses_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 2);
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
pub fn damage_each_opponent(g: &mut dyn Game, item: &StackItem, n: u32) {
    let hits: Vec<(ObjectId, Target, u32)> = g.state().opponents(item.controller).map(|p| (item.object, Target::Player(p), n)).collect();
    g.damage_together(&hits);
}

/// "[This source] deals `n` damage to each opponent and you gain `n` life" (CR 120.3, 119.3): `damage_each_opponent`, then the
/// controller gains the life whether or not the damage was dealt (CR 608.2c). Extended Absence, Screeching Soulbreaker, Eye of
/// Jace, Aurelia, the Law Above, Quintorius Kand, Sanctum Lurker, Ark of Hunger and Colossus of the Blood Age print it.
pub fn damage_each_opponent_and_gain_life(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    damage_each_opponent(g, item, n);
    g.gain_life(item.controller, n as i32);
    Ok(())
}

/// "This source deals N damage to that permanent's controller", the permanent being the trigger's subject: its controller
/// as the ability resolves, or as it last existed if it has left the battlefield (CR 608.2h), with no target and no
/// retest of the trigger condition. The damage is dealt by the ability's source, as it last existed if it has left
/// (CR 113.7a); none is dealt to a player who has left the game (CR 102.1, 120.1, 800.4a), which the core's damage sees to. Rampaging
/// Ferocidon, Tetsuko Umezawa, Pursuer, Ankh of Mishra (1), Dingus Egg and Psychic Venom (2) print it.
pub fn ping_subject_controller<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(subject) = item.subject.object() {
        let controller = g.controller(subject);
        g.damage(item.object, Target::Player(controller), N);
    }
    Ok(())
}

/// Compatibility name for `ping_subject_controller`: the trigger's permanent subject identifies the controller.
pub fn damage_subject_controller<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    ping_subject_controller::<N>(g, item)
}

/// "This source deals 1 damage to you" (CR 120.3, 608.2h): use the ability's controller.
/// City of Brass, Serendib Efreet, Mana Vault, Callous Inspector and Chandra's emblem print it.
pub fn source_deals_one_to_controller(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, Target::Player(item.controller), 1);
    Ok(())
}

/// At the beginning of enchanted permanent's controller's upkeep, this Aura deals N damage to that player.
/// Cursed Land, Feedback, Wanderlust and Warp Artifact print it (CR 503.1a, 608.2h).
/// Pair the matcher and effect so its host subject and saved upkeep player have one shared contract.
pub const fn enchanted_permanents_upkeep_damage<const N: u32>(text: &'static str) -> AbilityDef {
    triggered(crate::triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST, damage_upkeep_player::<N>, text)
}

fn damage_upkeep_player<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, Target::Player(PlayerId(item.data[0] as u8)), N);
    Ok(())
}

/// "Surveil N" (CR 701.25) for the ability's controller. The surveil lands and Dragon's Rage Channeler print it.
pub fn surveil(g: &mut dyn Game, item: &StackItem, n: usize) -> Result<(), Suspend> {
    crate::surveil(g, item.controller, item.object, n)
}

/// "Reveal the top card of your library. Draw a card if it's an artifact card." (CR 701.20a, 121.1): the card revealed
/// stays on top unless it is an artifact, in which case it is the card drawn. `may` makes the revealing its controller's
/// choice ("you may reveal it"); an empty library reveals nothing and a nonartifact card is left on top, so neither
/// draws. Says whether a card was drawn, for text that does something else when it wasn't ("If you don't draw a card this
/// way"). Iron Lad, Diverging Destiny reveals it, B'Elanna Torres, Scrapper may.
pub fn reveal_top_and_draw_if_artifact(g: &mut dyn Game, item: &StackItem, may: Option<&'static Question>) -> Result<bool, Suspend> {
    let player = item.controller;
    let Some(&top) = crate::top_of_library(g, player, 1).first() else { return Ok(false) };
    if let Some(question) = may {
        if !ask_may(g, player, item.object, Purpose::Ask(question))? {
            return Ok(false);
        }
    }
    g.reveal(player, &[top]);
    if !g.chars(top).is(CardType::Artifact) {
        return Ok(false);
    }
    // A replacement effect (dredge, CR 702.52a) can take the draw's place; "didn't draw" is asked of the history (CR 614.6).
    let drawn_before = g.history().drawn(player);
    g.draw(player, 1);
    Ok(g.history().drawn(player) != drawn_before)
}

/// "Choose a card name" / "choose a land card name" (CR 201.4, 201.4a) as an ability resolves: the controller names a card of
/// `kind` (an unscripted player the `suggestion`), and the name is remembered by the ability's source as the object it is (CR
/// 607.2d), for the rest of its text to refer to (`GameState::chosen_card`). The name chosen, if one was; if the source has left
/// the battlefield by then the choice is made and nothing keeps it. Medomai's Prophecy and Petrified Hamlet print it.
pub fn choose_a_card_name(g: &mut dyn Game, item: &StackItem, kind: NameKind, suggestion: Option<CardId>) -> Result<Option<CardId>, Suspend> {
    let ask = RequestKind::ChooseName { source: item.object, looked_at: Vec::new(), suggestion, kind };
    let Answer::Name(Some(name)) = g.ask(item.controller, ask)? else { return Ok(None) };
    g.remember_chosen_name(g.linked(item), name);
    Ok(Some(name))
}

/// "Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle." (CR 701.23): only a card
/// with the basic supertype counts (CR 205.4c), Plains is read through the source's words (CR 612), and a search for a quality
/// may find nothing (CR 701.23b). Gloryheath Lynx, Oberth-Class Starship and The Birth of Meletis print it as a whole effect.
pub fn search_for_a_basic_plains_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::library::find_to_hand(g, item.controller, item.object, crate::select::BASIC_PLAINS_CARD, true)?;
    Ok(())
}

/// "As this permanent enters, choose a nonland card name." (CR 614.12a, 201.3): recorded as the entering object's chosen name,
/// for its other text to refer to (`GameState::chosen_card`). Kai Winn, Faithless Leader and Skyseer's Chariot print it.
pub fn as_this_enters_choose_a_nonland_card_name(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let ask = RequestKind::ChooseName { source: entering.object, looked_at: Vec::new(), suggestion: None, kind: NameKind::NONLAND };
    if let Answer::Name(name) = g.ask(entering.controller, ask)? {
        entering.chosen_card = name;
    }
    Ok(())
}

/// "Return target <permanent> to its owner's hand. Surveil N." as one spell's whole effect, for its first target.
/// Banishing Betrayal (nonland permanent, surveil 1) and Ephara's Dispersal (creature, surveil 2) print it. A spell whose
/// only target has become illegal does nothing at all (CR 608.2b), so there is no surveil then; otherwise the surveil
/// follows whether or not the permanent actually moved.
pub fn return_first_target_to_hand_then_surveil<const N: usize>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[permanent]);
    g.end_sentence();
    surveil(g, item, N)
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

/// "Each opponent discards a card" (CR 701.9, 101.4), and the opponents who couldn't, for a clause that follows: "for each
/// opponent who can't, ..." or "each opponent who can't ...". An opponent "can't" with no card in hand, or when nothing lets
/// this ability make them discard (CR 101.2: Tamiyo, Collector of Tales); that is judged before they discard, the discards
/// are made together, and the opponents who couldn't are returned in turn order. Aclazotz, Deepest Betrayal (draws for each) and
/// Entropic Battlecruiser (loses 3 life) print it.
pub fn each_opponent_discards_a_card_and_those_who_cant(g: &mut dyn Game, item: &StackItem) -> Result<Vec<PlayerId>, Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != you).collect();
    let cant: Vec<PlayerId> = opponents
        .iter()
        .copied()
        .filter(|&p| g.state().zone(Zone::hand(p)).is_empty() || !may_cause(&*g, you, p, CauseKind::Discard))
        .collect();
    each_opponent_discards_a_card(g, item)?;
    Ok(cant)
}

/// "[Source] deals N damage to target [thing]" (CR 120.3): the source of the resolving item deals N damage to its first
/// target, if that is still a legal target as it resolves (CR 608.2b); a source that has left its zone (sacrificed as a cost,
/// discarded, dead) deals it as it last existed (CR 113.7a, 608.2h). Which targets are allowed is the card's own spell or
/// ability. Lightning Bolt (any target, 3), Shock (2), Flametongue Kavu (creature, 4) and Hired Claw (opponent, 1) print it.
pub fn damage_first_target<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::damage_target(g, item, 0, N);
    Ok(())
}

/// "It deals damage equal to the number of creatures you control to target [creature]" (CR 120.3, 608.2h): the creatures counted as
/// the ability resolves, so a creature sacrificed for its cost is not in the count (ruling 2025-04-04). Which targets
/// are allowed is the card's own. Stadium Headliner and Team Avatar print it.
pub fn damage_first_target_by_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = crate::creatures_you_control(g, item.controller);
    crate::damage_target(g, item, 0, creatures as u32);
    Ok(())
}

/// "Counter target spell unless its controller pays {N}" (CR 701.6, 118.12): the spell that is the first target is countered
/// unless its controller pays the generic cost, which they may make mana for while asked; nothing if it is no longer a legal
/// target as this resolves (CR 608.2b). Which spells may be targeted is the card's own. Mana Leak ({3}), Daze ({1}) and Spell
/// Pierce ({2}) print it.
pub fn counter_first_target_unless_pays<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::counter_unless_pays(g, item, 0, generic(N as u32))?;
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

/// "Target player mills N cards" (CR 701.17a): the player the item targeted first, if it is still a legal target as it resolves
/// (CR 608.2b). Breaking // Entering (eight), Ipnu Rivulet and Restless Reef (four), Masters & Councillors (three) and
/// Scavenger's Talent (two) print it.
pub fn target_player_mills<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        g.mill(player, N);
    }
    Ok(())
}

/// "Each opponent mills N cards" (CR 701.17a, 101.4): every opponent of the controller mills together, as one instruction.
/// Aether Syphon and Psychic Corrosion print it (two).
pub fn each_opponent_mills<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mills: Vec<(PlayerId, u32)> = g.state().opponents(item.controller).map(|p| (p, N)).collect();
    g.mill_together(&mills);
    Ok(())
}

/// "You draw two cards and each player loses 2 life." (CR 121.1, 119.3, 101.4): the controller draws, then every player in
/// the game loses the life together, as one event, replaced in turn order from the active player. Impossible Choice's second mode and
/// The City on the Edge of Forever's chapter II print it.
pub fn draw_two_each_player_loses_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    // Every player loses it together: one event, replaced in APNAP order (CR 101.4, 603.2c).
    let losses: Vec<(PlayerId, i32)> = g.state().apnap().map(|player| (player, 2)).collect();
    g.lose_life_together(&losses);
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

/// "Destroy up to N target [things]" (CR 701.8, 608.2b, 101.4): every object target that is still legal, destroyed together,
/// so none is gone before another is destroyed. Force of Vigor and Armaggon, Future Shark print it.
pub fn destroy_each_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.destroy_all(&crate::targets::legal_objects(g, item));
    Ok(())
}

/// "Destroy target [thing]" (CR 701.8): the ability's first target.
pub fn destroy_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::destroy_target(g, item, 0);
    Ok(())
}

const MAY_DESTROY_TARGET: Question = Question::new("Destroy", "destroy the targeted permanent?", "", "Destroy it", "Don't destroy");

/// "You may destroy target [permanent]" (CR 701.8, 608.2b, 603.3d): the target is chosen as the ability goes on the stack and
/// the "may" is asked as it resolves, only if the first target is still legal. Aura Shards and Reclamation Sage print it.
pub fn may_destroy_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = crate::targets::object(g, item, 0) else { return Ok(()) };
    if ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_DESTROY_TARGET))? {
        g.destroy(permanent);
    }
    Ok(())
}

/// "Destroy target [permanent]. Its controller creates a [token]" (CR 701.8, 608.2h): the ability's first target, its
/// controller read before the destroy, and the token created whether or not it was destroyed (indestructible, regenerated);
/// an illegal target creates nothing (CR 608.2b). Stroke of Midnight and Bovine Intervention print it.
pub fn destroy_and_its_controller_creates(g: &mut dyn Game, item: &StackItem, token: &'static CardDef) -> Result<(), Suspend> {
    destroy_then_its_controller_creates(g, item, token, true)
}

/// "Destroy target [permanent]. It can't be regenerated. Its controller creates a [token]" (CR 701.8, 701.19c, 608.2h):
/// `destroy_and_its_controller_creates` with a destroy a regeneration shield does not save. Pongify and Rapid
/// Hybridization print it.
pub fn destroy_no_regen_and_its_controller_creates(g: &mut dyn Game, item: &StackItem, token: &'static CardDef) -> Result<(), Suspend> {
    destroy_then_its_controller_creates(g, item, token, false)
}

fn destroy_then_its_controller_creates(g: &mut dyn Game, item: &StackItem, token: &'static CardDef, regenerable: bool) -> Result<(), Suspend> {
    let Some(target) = crate::targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(target);
    if regenerable {
        g.destroy(target);
    } else {
        g.destroy_no_regen(target);
    }
    g.end_sentence();
    crate::tokens::create(g, controller, token, 1)?;
    Ok(())
}

/// "{N}, Sacrifice this creature: Destroy target artifact or enchantment" (CR 602.2, 701.8, 118.3): the sacrifice is a
/// cost paid as the ability is activated and the destroy needs a target. Cathar Commando, Outland Liberator (and its back
/// face) and Shattered Acolyte print it with {1}, Kirk, Commanding Officer with {2}.
pub const fn sacrifice_to_destroy_artifact_or_enchantment(generic_mana: u8, text: &'static str) -> AbilityDef {
    ability(Cost { mana: generic(generic_mana as u32), sacrifice_self: true, ..Cost::NONE }, destroy_first_target, text)
        .targets(&[crate::targets::ARTIFACT_OR_ENCHANTMENT])
}

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
    // Each card of a merged permanent (CR 730.3c), of which only a double-faced one returns (CR 712.14a).
    for card in g.found_all_after_move(item.object) {
        let owner = g.obj(card).owner;
        g.put_onto_battlefield_transformed(card, owner, true)?;
    }
    Ok(())
}

/// "Exile it" for a "when this dies" ability: the card the permanent became in the graveyard, if it is still there (CR 400.7e) -
/// each card of a merged permanent, exiled together (CR 730.3c, 101.4). The objects now in exile; a replacement may have sent one
/// elsewhere. Ancient Adamantoise, Cavalier of Thorns, Paramecia Coloniex and Serra Paragon print it.
pub fn exile_what_it_became(g: &mut dyn Game, left: ObjectId) -> Vec<ObjectId> {
    let cards = g.found_all_after_move(left);
    g.exile_all(&cards).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect()
}

/// "Return it to the battlefield under its owner's control" for a "when this dies" ability: the card the permanent became in the
/// graveyard, if it is still there (CR 400.7e) - each card of a merged permanent, together (CR 730.3c, 101.4) - tapped if
/// `tapped`, each entering with `counters` (CR 122.6). The permanents that entered; a replacement may have kept one out. Hellcat,
/// Retched Wretch, Pain 101, Unstoppable Slasher, persist and undying print it.
pub fn return_what_it_became(g: &mut dyn Game, left: ObjectId, tapped: bool, counters: &[(CounterKind, i32)]) -> Result<Vec<ObjectId>, Suspend> {
    let cards: Vec<(ObjectId, PlayerId, bool)> = g.found_all_after_move(left).into_iter().map(|c| (c, g.obj(c).owner, tapped)).collect();
    if cards.is_empty() {
        return Ok(Vec::new());
    }
    Ok(g.put_all_onto_battlefield_with_counters(&cards, counters)?.into_iter().filter(|&c| g.is_on_battlefield(c)).collect())
}

/// "You may pay [cost]. If you do, untap this [permanent]" (CR 608.2c, 701.26).
/// Brass Man and Mana Vault print this; the payment is offered even if the source has left.
pub fn may_pay_to_untap_self(g: &mut dyn Game, item: &StackItem, payment: Cost) -> Result<(), Suspend> {
    static QUESTION: Question = Question::new("UntapSource", "pay {cost} to untap {subject}?", "", "Pay {cost}", "Decline");
    let paid = g.may_pay(item.controller, item.object, item.object, payment, &QUESTION)?;
    g.end_sentence();
    if paid && crate::targets::permanent(g, item.object) {
        g.untap(item.object);
    }
    Ok(())
}

/// "You may pay {3}. If you do, untap target permanent" (CR 118.12, 608.2b, 608.2c, 701.26).
/// Hematite Talisman and Onyx Talisman print this. The sole target was checked when resolution began;
/// mana abilities used to pay can change its legality or remove its incarnation before the untap.
pub fn may_pay_three_to_untap_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    static QUESTION: Question = Question::new("Pay", "pay {cost} to untap target permanent?", "", "Pay {cost}", "Decline");
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{3}"), &QUESTION)?;
    g.end_sentence();
    if paid {
        if let Target::Object(target) = g.state().target(item, 0) {
            g.untap(target);
        }
        g.end_sentence();
    }
    Ok(())
}

/// "Sacrifice this [permanent] unless you pay [cost]" (CR 608.2c, 701.21a, 118.1): the resolving ability's controller pays
/// `payment` or sacrifices the source. Arcades Sabboth and Phantasmal Forces print it at the beginning of their controller's upkeep.
pub fn sacrifice_this_unless_pay(g: &mut dyn Game, item: &StackItem, payment: Cost) -> Result<(), Suspend> {
    static QUESTION: Question = Question::new("Pay", "pay {cost}?", "If you don't, sacrifice {subject} if you control it", "Pay {cost}", "Don't pay").no_default();
    if !g.may_pay(item.controller, item.object, item.object, payment, &QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "Untap this [permanent]" (CR 701.26): the ability's source. Basalt Monolith and Grim Monolith print it.
pub fn untap_self(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.untap(item.object);
    Ok(())
}

/// "Untap all lands you control" (CR 701.26b): every land the ability's controller controls as it resolves, with no choice.
/// Emrakul, the Exigent Doom's cast trigger and Zacama, Primal Calamity's enters trigger print it.
pub fn untap_all_lands_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    g.untap_together(&lands);
    Ok(())
}

/// "Untap all creatures you control" (CR 701.26b): every creature the ability's controller controls as it resolves, with no
/// choice, untapped together (CR 101.4). Merciful Mandate prints it.
pub fn untap_all_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.untap_together(&creatures);
    Ok(())
}

/// "Untap all nonland permanents you control" (CR 701.26b): everything the ability's controller controls as it resolves that is
/// not a land, with no choice, untapped together (CR 101.4). Paradox Engine and Unstoppable Plan print it.
pub fn untap_all_nonland_permanents_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let nonland: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| !g.chars(id).is(CardType::Land)).collect();
    g.untap_together(&nonland);
    Ok(())
}

/// "Put a +1/+1 counter on each of up to N target creatures" (CR 122.1, 601.2c, 608.2b): on every target of the ability that is still
/// legal as it resolves, put on together (CR 101.4), so a "one or more" ability triggers once. Agent Bishop, Man in Black and White
/// Widow, Free Agent print it.
pub fn plus_one_counter_on_each_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i32)> =
        crate::targets::legal_objects(g, item).into_iter().map(|creature| (creature, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

/// "Tap target [permanent]. Put `N` stun counters on it." (CR 701.26a, 122.1d): the counter goes on whether or not the
/// permanent was untapped, so a tapped one gets it too; an illegal target does nothing (CR 608.2b). The target filter stays with
/// each card ("up to one" included). With 1 counter: Loch Mare's second ability, Sygg's Command's fourth mode, Champions of the
/// Shoal, Consider the Prime Directive's Observe, Deluge Virtuoso, Floodpits Drowner and Utrom Scientists. With 2: Kaito, Bane of Nightmares and Lodestone Needle.
pub fn tap_and_stun<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = crate::targets::object(g, item, 0) {
        g.tap(permanent);
        g.add_counters(permanent, CounterKind::Stun, N);
    }
    Ok(())
}

/// "Tap [up to one] target creature and put a stun counter on it. Draw a card." (CR 701.26a, 122.1d, 121.1): Cryogenic Stasis and Consider the Prime Directive.
pub fn tap_and_stun_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tap_and_stun::<1>(g, item)?;
    draw(g, item, 1)
}

/// "Tap all creatures your opponents control" (CR 701.26a, 101.4): every creature each opponent of the resolving item's
/// controller controls as the effect is carried out, tapped together; an already-tapped one stays tapped. Cryptic Command,
/// Ultimecia, Temporal Threat and Summon: Alexander (Crystal Fragments' back face) print it.
pub fn tap_each_creature_opponents_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let creatures: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && opponents.contains(&g.controller(id)))
        .collect();
    g.tap_together(&creatures);
    Ok(())
}

/// "Tap [N] target [permanents]" (CR 701.26a, 608.2b): each target that is still legal as the ability resolves. Divining
/// Duelist (as a mode), Quake, Agent of S.H.I.E.L.D. and Raft Security Officer print it.
pub fn tap_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = crate::targets::legal_objects(g, item);
    g.tap_together(&permanents);
    Ok(())
}

/// "Untap [N] target [permanents]" (CR 701.26, 608.2b): each target that is still legal as the ability resolves. Argothian
/// Elder, Candelabra of Tawnos and Garruk, Curse Breaker (for lands), Formidable Speaker, Kiora, Behemoth Beckoner, Voltaic Key, Manifold Key, Earthcraft, Hapatra, the Desert Frost and Deceiver Exarch and Divining Duelist (as a mode) print it.
pub fn untap_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = crate::targets::legal_objects(g, item);
    g.untap_together(&permanents);
    Ok(())
}

/// "Sacrifice it. When you do, ..." (CR 701.21a, 603.12): this permanent is sacrificed, and the card's `delayed` ability number 0
/// triggers as a reflexive ability only if it really was (it had already gone, or could not be sacrificed: nothing). Claim the
/// Kingdom, Death to Our Enemies, Doom Reigns Supreme, Rewrite History and Villainous Syndication print it after "When the fourth
/// plan counter is put on this".
pub fn sacrifice_this_when_you_do(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.sacrifice(item.controller, &[item.object]).is_empty() {
        return Ok(());
    }
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), Target::None);
    Ok(())
}

/// "The owner of [a permanent] puts it into their library second from the top or on the bottom" (CR 401.4, 400.3): its owner,
/// not the ability's controller, chooses which, and it moves as it stands on the battlefield. The card lists the two options
/// in its `labels`, "Second from the top" first and "On the bottom" second. Wan Shi Tong, All-Knowing and Trickster's
/// Stratagem print it.
pub fn owner_puts_into_library_second_from_top_or_on_the_bottom(g: &mut dyn Game, item: &StackItem, permanent: ObjectId) -> Result<(), Suspend> {
    let owner = g.obj(permanent).owner;
    let card = g.obj(item.object).card;
    let second_from_top = crate::choose::one_of(g, owner, item.object, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0;
    g.put_into_library(&[permanent], if second_from_top { LibraryPlace::FromTop(1) } else { LibraryPlace::Bottom });
    Ok(())
}

/// "You may [do something]" (CR 117.3, 609.3): asks `player` whether to, for `purpose`, and says whether they did. The
/// question is the usual `May`, so a scenario answers it with `Accept` or `Decline`. Ad Nauseam, Cori-Steel Cutter and Currency Converter ask
/// it.
pub fn ask_may(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose) -> Result<bool, Suspend> {
    Ok(g.ask(player, RequestKind::May { source, purpose })? == Answer::Yes)
}

const PAY_TO_GAIN_LIFE: Question = Question::new("GainLife", "pay {cost} to gain 1 life?", "", "Pay {cost}", "Don't pay");

/// "You may pay [cost]. If you do, you gain 1 life." (CR 608.2d, 118.12, 119.3) as the ability resolves: its controller is asked
/// whether to pay `cost`, and gains 1 life only if they did. Farmstead, Ivory Cup and Soul Net print it.
pub fn may_pay_to_gain_life(g: &mut dyn Game, item: &StackItem, cost: Cost) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY_TO_GAIN_LIFE)? {
        g.gain_life(item.controller, 1);
    }
    Ok(())
}

const PAY_TO_RETURN_THIS_TO_HAND: Question =
    Question::new("ReturnToHand", "pay {cost} to return {subject} to your hand?", "It is in your graveyard", "Pay {cost}", "Leave it");

/// "You may pay [cost]. If you do, return this card from your graveyard to your hand." (CR 608.2d, 118.12, 400.3) as the
/// ability resolves: its controller is asked whether to pay `cost`, and the card returns only if they did; a card that has left
/// the graveyard by then is a new object and is not returned (CR 400.7). Killian's Confidence, Gabriel Lorca, Mirror Menace,
/// Shambling Cie'th and Furious Forebear print it.
pub fn may_pay_to_return_this_to_hand(g: &mut dyn Game, item: &StackItem, cost: Cost) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY_TO_RETURN_THIS_TO_HAND)? {
        return_this_to_hand(g, item)?;
    }
    Ok(())
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
    if g.cant_draw(player, n) || !ask_may(g, player, source, Purpose::Ask(question))? {
        return Ok(false);
    }
    g.draw(player, n);
    Ok(true)
}

const MAY_DRAW_A_CARD: Question = Question::new("Draw", "draw a card?", "", "Draw", "Don't draw");

const MAY_DRAW_THAT_MANY_ONCE: Question = Question::new("Draw", "draw that many cards?", "One for each +1/+1 counter put  ·  only once each turn", "Draw", "Don't draw");

/// "You may draw that many cards. Do this only once each turn." (CR 603.2h, 121.3): the effect of a triggered ability whose
/// `item.data[0]` is the number of +1/+1 counters just put, so it offers to draw that many. Asked only while its source has
/// not drawn with this ability this turn, and spent only once a card was actually drawn, not by a yes whose draws a
/// replacement took (CR 614.6, 603.2h); not offered to a player who can't draw that many (CR 121.3,
/// 121.2b). Terrasymbiosis (counters put on a creature you control) and Chekov and McCoy (on itself) print it.
pub fn may_draw_that_many_once_each_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.data[0] == 0 || g.history().done(item.object, item.ability, item.controller) {
        return Ok(());
    }
    let before = g.history().drawn(item.controller);
    if may_draw(g, item.controller, item.object, item.data[0], &MAY_DRAW_THAT_MANY_ONCE)? && g.history().drawn(item.controller) > before {
        g.note_done(item.object, item.ability, item.controller);
    }
    Ok(())
}

/// "You may draw a card" (CR 121.3, 608.2d): asked of the ability's controller, not offered a player who can't draw a card
/// (`may_draw`). Leovold, Emissary of Trest and Mister Fantastic, Reed Richards print it.
pub fn may_draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    may_draw(g, item.controller, item.object, 1, &MAY_DRAW_A_CARD)?;
    Ok(())
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

const MAY_MILL_A_CARD: Question = Question::new("Mill", "mill a card?", "The top card of your library goes into your graveyard", "Mill", "Don't mill");
const MAY_MILL_TWO: Question = Question::new("Mill", "mill two cards?", "The top two cards of your library go into your graveyard", "Mill two", "Don't mill");
const MAY_MILL_THREE: Question = Question::new("Mill", "mill three cards?", "The top three cards of your library go into your graveyard", "Mill three", "Don't mill");
const MAY_MILL_CARDS: Question = Question::new("Mill", "mill cards?", "That many cards from the top of your library go into your graveyard", "Mill", "Don't mill");

/// "You may mill `n` cards" (CR 701.17a, 701.17b): asked of the ability's controller, then the mill. A player can't choose to mill
/// more cards than their library holds, so with fewer than `n` cards in it nothing is asked and nothing is milled. Mole Man's
/// Moloid tokens and Patchwork Beastie (one card), Visage of Dread (two), Rainveil Rejuvenator (three) and Lasyd Prowler (the
/// lands they control) print it.
pub fn may_mill(g: &mut dyn Game, item: &StackItem, n: usize) -> Result<(), Suspend> {
    if g.state().zone(Zone::library(item.controller)).len() < n {
        return Ok(());
    }
    let question = match n {
        1 => &MAY_MILL_A_CARD,
        2 => &MAY_MILL_TWO,
        3 => &MAY_MILL_THREE,
        _ => &MAY_MILL_CARDS,
    };
    if ask_may(g, item.controller, item.object, Purpose::Ask(question))? {
        g.mill(item.controller, n as u32);
    }
    Ok(())
}

/// "You may mill a card" (CR 701.17a): `may_mill` of one card, for an ability's whole effect.
pub fn may_mill_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    may_mill(g, item, 1)
}

/// "You may [do something with] this card" (CR 113.6k): `ask_may` for a card that triggered from a zone it
/// functions in, asked only while the card is still the object that triggered (CR 400.7). Once it has left, nothing is asked
/// and the answer is no. Squee, Goblin Nabob and Vengevine ("you may return this card from your graveyard") and Creeping
/// Chill ("you may exile it") ask it.
pub fn ask_may_this_card(g: &mut dyn Game, item: &StackItem, purpose: Purpose) -> Result<bool, Suspend> {
    if !is_live(g, item.object) {
        return Ok(false);
    }
    ask_may(g, item.controller, item.object, purpose)
}

const RETURN_TO_BATTLEFIELD: Question = Question::new("ReturnToBattlefield", "return it from your graveyard to the battlefield?", "", "Return it", "Leave it");

/// "You may return this card from your graveyard to the battlefield" (CR 113.6k): `ask_may_this_card` with the
/// question worded for the battlefield, and the card put onto the battlefield untapped under its controller on a yes.
/// Nothing is asked once it is no longer the object that triggered (CR 400.7). Vengevine and Bloodghast print it.
pub fn may_return_this_card_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_this_card(g, item, Purpose::Ask(&RETURN_TO_BATTLEFIELD))? {
        g.put_onto_battlefield(item.object, item.controller, false)?;
    }
    Ok(())
}

/// "Return this card from your graveyard to the battlefield" and "Put this card from your hand onto the battlefield" (CR
/// 400.7): the ability's own source, put onto the battlefield untapped under its controller; nothing if it is no longer the
/// object the ability came from (the core skips a card that changed zones). Darklight Phoenix and Cauldron Familiar print
/// the first, Urban Retreat and Talon Gates of Madara the second.
pub fn return_this_card_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_onto_battlefield(item.object, item.controller, false)?;
    Ok(())
}

/// "Return this card from your graveyard to the battlefield with a finality counter on it" (CR 400.7, 122.6, 614.1c): the
/// ability's own source, put onto the battlefield untapped under its controller with the counter already on it, so it is
/// exiled instead of dying (CR 122.1h). The new object, for an ability that goes on to do something with it. Winter Soldier,
/// Icy Assassin prints it.
pub fn put_this_card_onto_battlefield_with_finality(g: &mut dyn Game, item: &StackItem) -> Result<ObjectId, Suspend> {
    g.put_onto_battlefield_with_counters(item.object, item.controller, false, &[(CounterKind::Finality, 1)])
}

/// "Return this card from your graveyard to the battlefield with a finality counter on it" and nothing more. Grim Repriser
/// and Proctor of Potential print it.
pub fn return_this_card_with_finality(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    put_this_card_onto_battlefield_with_finality(g, item)?;
    Ok(())
}

/// "Return this card from your graveyard to the battlefield tapped" (CR 400.7): `return_this_card_to_battlefield`, entering
/// tapped. Otharri, Suns' Glory and Teacher's Pest print it.
pub fn return_this_card_to_battlefield_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.put_onto_battlefield(item.object, item.controller, true)?;
    Ok(())
}

/// "Return this [permanent or card] to its owner's hand" (CR 400.3): the ability's own source, from the battlefield or a
/// graveyard. Nothing if it is no longer the object it was by the time the ability resolves (CR 400.7). Altar of the Wretched
/// and Puppet Crafting ("from your graveyard") and Bramble Familiar ("this creature") print it.
pub fn return_this_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.return_to_hand(&[item.object]);
    Ok(())
}

/// "Return it [that card] to its owner's hand" (CR 400.3) for the card a dies or put-into-graveyard trigger is about
/// (`StackItem::subject`, the new object in the graveyard, CR 400.7): returned only if it is still there. Rancor and Ares,
/// God of War print it.
pub fn return_subject_from_graveyard_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(card) = item.subject {
        let o = g.obj(card);
        if o.became.is_none() && o.zone == Zone::graveyard(o.owner) {
            g.return_to_hand(&[card]);
        }
    }
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

const MAY_RETURN_TARGET_TO_HAND: Question = Question::new("ReturnToHand", "return the targeted card to your hand?", "", "Return it", "Leave it");

/// "You may return target card from your graveyard to your hand" (CR 608.2b, 117.3): asked as the ability resolves, and only
/// if the first target is still legal. Eternal Witness and Graveshifter print it.
pub fn may_return_first_target_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = crate::targets::object(g, item, 0) else { return Ok(()) };
    if ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_RETURN_TARGET_TO_HAND))? {
        g.return_to_hand(&[card]);
    }
    Ok(())
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

const MAY_RETURN_TARGET_TO_BATTLEFIELD: Question = Question::new("ReturnToBattlefield", "return the targeted card to the battlefield?", "", "Return it", "Leave it");

/// "You may return target [permanent] card from your graveyard to the battlefield" (CR 608.2b, 117.3, 110.2a): the target is
/// mandatory, chosen as the ability goes on the stack; the "may" is asked as it resolves, and only if the target is still
/// legal. The card enters untapped under the ability's controller. Sun Titan and Bishop of Rebirth print it; Eternal
/// Witness's `may_return_first_target_to_hand` is its sibling.
pub fn may_return_first_target_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = crate::targets::object(g, item, 0) else { return Ok(()) };
    if ask_may(g, item.controller, item.object, Purpose::Ask(&MAY_RETURN_TARGET_TO_BATTLEFIELD))? {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

/// "Return up to one target [permanent] to its owner's hand. If you do, put a +1/+1 counter on this creature" (CR 400.3, 122.1): a
/// target that became illegal is not returned (CR 608.2b); the source gets the counter only if the permanent really arrived in a
/// hand ("if a permanent was returned this way"), and only if it is still on the battlefield. Mirkwood Nurturer and Nurturing Pixie
/// print it.
pub fn return_first_target_to_hand_then_plus_one_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = crate::targets::object(g, item, 0) else { return Ok(()) };
    if !g.return_to_hand(&[permanent]).is_empty() {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// "Exile [up to one] target [object]" (CR 701.13a, 608.2b): Sela, Tal Shiar Paradox, Exorcise and Soul-Guide Lantern.
pub fn exile_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::exile_target(g, item, 0);
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

/// "The exiled card's owner creates an X/X [token], where X is the mana value of the exiled card" (CR 607.2a, 111.4): the
/// card exiled with the ability's source, as `exile_first_target_with_this` or `exile_with` linked it; with none exiled,
/// no one creates anything. The token is `token` X/X as it is made (CR 111.4: the effect that creates it defines its
/// characteristics, and those are its copiable values, CR 707.2), never a 0/0 for a moment, so what triggers on it
/// entering sees an X/X. Skyclave Apparition (a blue Illusion) and Severance Priest (a white Spirit) print it.
pub fn exiled_cards_owner_creates_x_x(g: &mut dyn Game, item: &StackItem, token: &'static CardDef) -> Result<(), Suspend> {
    let Some(&card) = g.exiled_with(g.linked(item)).first() else { return Ok(()) };
    let owner = g.obj(card).owner;
    let x = g.mana_value(card) as i32;
    let values = Copiable::of(g.card_id(token)).sized(x, x);
    g.create_token_copies_of(values, owner, 1)?;
    Ok(())
}

/// "Return target [card] from your graveyard to the battlefield" (CR 400.3): the first target, under the controller of the
/// spell or ability, untapped; nothing if it is no longer a legal target (CR 608.2b), which for a lone required target
/// means the spell or ability does not resolve at all. Emptiness, Ral Zarek, Guest Lecturer's −2, Primary Research, Recurring Nightmare,
/// Restoration Seminar and Unearth print it.
pub fn return_first_target_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = crate::targets::object(g, item, 0) {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

/// "Return [up to N] target [creature cards] from your graveyard to the battlefield. Put a +1/+1 counter on each of them" (CR 400.3,
/// 122.1): the targets still legal enter under your control (CR 608.2b, 110.2a), all at once; then, as the next sentence (CR 608.2c),
/// one +1/+1 counter goes on each that did enter - a new object each (CR 400.7), one instruction (CR 122.6). A card a replacement
/// effect sent elsewhere (Containment Priest's exile) never became one of them. Scout for Survivors and Smile at Death print it.
pub fn return_targets_to_battlefield_with_plus_one_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<(ObjectId, PlayerId)> = crate::targets::legal_objects(g, item).into_iter().map(|c| (c, you)).collect();
    let entered = g.put_all_onto_battlefield(&cards)?;
    g.end_sentence();
    let counters: Vec<(ObjectId, CounterKind, i32)> = entered.into_iter().filter(|&id| g.is_on_battlefield(id)).map(|id| (id, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&counters);
    Ok(())
}

/// "This source deals damage to any target and damage to you" (CR 608.2b-c, 120.4b): one instruction, both hits simultaneous.
/// Psionic Blast and Spicy Oatmeal Pizza use this recipe; the resolving item has one mandatory target.
pub fn damage_target_and_you<const TARGET: u32, const YOU: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = g.state().target(item, 0);
    g.damage_together(&[(item.object, target, TARGET), (item.object, Target::Player(item.controller), YOU)]);
    Ok(())
}

/// The source deals damage to a target and to itself in the same instruction (CR 608.2c, 120.4).
/// Psionic Entity and the ability granted by Psionic Sliver use this recipe.
pub fn damage_target_and_source<const TARGET: u32, const SOURCE: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = g.state().target(item, 0);
    g.damage_together(&[(item.object, target, TARGET), (item.object, Target::Object(item.object), SOURCE)]);
    Ok(())
}

/// "[This source] deals `damage` damage to [a target] and you gain `life` life" (CR 120.3, 119.1): the first target, if it is
/// still legal (CR 608.2b), is dealt the damage by the resolving object (as it last was if it has left, CR 113.7a). The gain,
/// a fixed amount that does not use the target, is made whenever this resolves: an item with that target alone never resolves
/// once it is illegal, and a mode of a modal spell still resolving for another mode's target gains it anyway (CR 608.2b), as
/// `target_player_loses_life_you_gain_life` does. Which targets are allowed is the card's own spell or ability. Lightning
/// Helix, Phlage, Titan of Fire's Fury and Demonic Pact print it with equal amounts; Knockout Blow deals 4 and gains 2.
pub fn damage_target_and_gain_life(g: &mut dyn Game, item: &StackItem, damage: u32, life: i32) -> Result<(), Suspend> {
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

/// "This source deals N damage to each creature and each player" (CR 608.2c, 120.4): all recipients
/// are damaged together, including both players and creatures controlled by either. Pestilence and Encounter the Unknown print it.
pub fn damage_each_creature_and_player<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_creature_and_player_n(g, item, N);
    Ok(())
}

/// `damage_each_creature_and_player` for an amount worked out as the effect resolves, not printed: Volcanic Eruption deals
/// damage equal to the number of Mountains put into a graveyard this way.
pub fn damage_each_creature_and_player_n(g: &mut dyn Game, item: &StackItem, n: u32) {
    let mut hits: Vec<(ObjectId, Target, u32)> =
        creatures_on_the_battlefield(g).into_iter().map(|id| (item.object, Target::Object(id), n)).collect();
    hits.extend(g.state().apnap().map(|p| (item.object, Target::Player(p), n)));
    g.damage_together(&hits);
}

/// "Target spell or permanent becomes [color]" (CR 611.2a, 613.1e): the target's colors are set to `COLOR`, read through the
/// words of the effect's text, indefinitely (CR 105.3: mana symbols are unchanged); it follows a permanent spell onto the
/// battlefield (CR 400.7a). Chaoslace, Deathlace, Lifelace, Purelace and Thoughtlace print it, each with its color.
pub fn target_becomes_color<const COLOR: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    let colors = g.words(item.object).colors(ColorSet::of(Color::ALL[COLOR as usize]));
    g.add_effect(item.object, object, EffectKind::SetColors(colors), Duration::Indefinite);
    Ok(())
}

/// "[This spell] deals `n` damage to each creature" (Pyroclasm, Fiery Confluence, Avengers Disassembled): one damage event
/// for every creature on the battlefield as the effect is carried out (CR 608.2c), all dealt together.
pub fn damage_each_creature(g: &mut dyn Game, item: &StackItem, n: u32) {
    damage_each_creature_dealt(g, item, n);
}

/// "[This spell] deals `n` damage to each creature your opponents control" (CR 608.2c, 120.2): one damage event for every
/// creature an opponent of the resolving item's controller controls as the effect is carried out, all dealt together.
/// Iroh's Demonstration and Easy Pickings (Glóin the Mighty) print it.
pub fn damage_each_creature_opponents_control(g: &mut dyn Game, item: &StackItem, n: u32) {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let hits: Vec<(ObjectId, Target, u32)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && opponents.contains(&g.controller(id)))
        .map(|id| (item.object, Target::Object(id), n))
        .collect();
    g.damage_together(&hits);
}

/// "[This spell] deals `n` damage to each opponent and each creature they control" (CR 608.2c, 120.2): one damage event for
/// each opponent of the resolving item's controller and each creature one controls, all dealt together. Delayed Blast
/// Fireball, Soul Immolation and Khan, Despot by Design print it.
pub fn damage_each_opponent_and_each_creature_they_control(g: &mut dyn Game, item: &StackItem, n: u32) {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let creatures: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && opponents.contains(&g.controller(id)))
        .collect();
    let hits: Vec<(ObjectId, Target, u32)> = opponents
        .iter()
        .map(|&p| Target::Player(p))
        .chain(creatures.into_iter().map(Target::Object))
        .map(|target| (item.object, target, n))
        .collect();
    g.damage_together(&hits);
}

/// "[This spell] deals `n` damage to each non-[subtype] creature" (CR 608.2c, 120.2): one damage event for every creature on
/// the battlefield without `subtype` as the effect is carried out, all dealt together. The caller reads the subtype through
/// the words of the object whose text names it (`g.words(item.object).subtype(..)`, CR 612). Desolation of Smaug (non-Dragon)
/// and Chandra, Awakened Inferno (non-Elemental) print it.
pub fn damage_each_creature_not_of_subtype(g: &mut dyn Game, item: &StackItem, subtype: Subtype, n: u32) {
    let hits: Vec<(ObjectId, Target, u32)> = g
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
pub fn damage_each_creature_and_planeswalker(g: &mut dyn Game, item: &StackItem, n: u32) {
    let hits: Vec<(ObjectId, Target, u32)> = g
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
pub fn damage_each_creature_dealt(g: &mut dyn Game, item: &StackItem, n: u32) -> Vec<ObjectId> {
    let hits: Vec<(ObjectId, Target, u32)> = g
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
pub fn damage_that_player(g: &mut dyn Game, item: &StackItem, n: u32) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        g.damage(item.object, Target::Player(p), n);
    }
    Ok(())
}

/// "Remove all counters from `permanent`" (CR 122.1, 122.8): every kind it has, all of each. Vampire Hexmage (a target) and
/// Enchanted River's Grasp (the enchanted creature) print it.
pub fn remove_all_counters(g: &mut dyn Game, permanent: ObjectId) {
    for kind in CounterKind::ALL {
        let n = g.obj(permanent).counters(kind);
        g.remove_counters(permanent, kind, n);
    }
}

/// "Target creature can't be blocked this turn" (CR 509.1b, 611.2): the ability's first target, if it is still a legal target
/// as the ability resolves (CR 608.2b). An effect on the creature, not an ability it gains (CR 113.1a). Elvenking's Harper,
/// Manifold Key, Eardrum Rattler and Escape Tunnel print it.
pub fn target_creature_cant_be_blocked_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::CantBeBlocked, Duration::EndOfTurn);
    }
    Ok(())
}

/// "This creature can't be blocked this turn" (CR 509.1b, 611.2): an effect on the source creature itself, for as long as it stays the
/// same permanent (CR 611.2c, 400.7). Matterbending Mage and Snooping Page print it.
pub fn this_cant_be_blocked_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::CantBeBlocked, Duration::EndOfTurn);
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
/// Earthbend, unearth and Chandra, Acolyte of Flame print it.
pub fn gain_haste(g: &mut dyn Game, source: ObjectId, object: ObjectId) {
    g.add_effect(source, object, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), Duration::Indefinite);
}

/// "`target` gains indestructible until end of turn" (CR 702.12b, 611.2a): a layer 6 keyword for the rest of the turn, from the
/// effect's source. Arwen, Mortal Queen, Cecil, Dark Knight, Crumb and Get It, Divine Resilience, Iron Shield Elf, Revitalizing Repast, Ursine
/// Monstrosity and Boros Charm print it.
pub fn gain_indestructible_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId) {
    gain_until_end_of_turn(g, source, target, Keyword::Indestructible);
}

/// "Target creature gains indestructible until end of turn" (CR 702.12b, 611.2a), the creature being the ability's first target;
/// nothing if it is no longer a legal target as the spell or ability resolves (CR 608.2b). Valorous Stance's first mode and
/// Burdened Stoneback print it.
pub fn target_creature_gains_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        gain_indestructible_until_end_of_turn(g, item.object, creature);
    }
    Ok(())
}

/// "Target creature gains double strike until end of turn" (CR 702.4, 611.2a), the creature (or other permanent) being the
/// ability's first target; nothing if it is no longer a legal target as the spell or ability resolves (CR 608.2b). Raking Claws,
/// Boros Charm's third mode and Pride of the Road's max speed ability print it.
pub fn target_gains_double_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = crate::targets::object(g, item, 0) {
        gain_until_end_of_turn(g, item.object, target, Keyword::DoubleStrike);
    }
    Ok(())
}

/// "This creature gains [a keyword] until end of turn. Tap it." (CR 611.2a, 701.26a): if the ability's source is still on the
/// battlefield as the permanent it was (CR 400.7), it gains the keyword for the rest of the turn, then it is tapped. The tap is
/// the ability's own instruction, not its cost, so the ability can be activated while it is tapped already (Underrealm Lich's
/// ruling). The two wrappers below are the fn pointers an ability's effect takes.
pub fn this_gains_then_tap(g: &mut dyn Game, item: &StackItem, keyword: Keyword) -> Result<(), Suspend> {
    let creature = item.object;
    if crate::targets::permanent(g, creature) {
        gain_until_end_of_turn(g, creature, creature, keyword);
        g.tap(creature);
    }
    Ok(())
}

/// "This creature gains indestructible until end of turn. Tap it." (CR 702.12b): `this_gains_then_tap` for indestructible.
/// Iron-Shield Elf and Underrealm Lich print it.
pub fn this_gains_indestructible_then_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_then_tap(g, item, Keyword::Indestructible)
}

/// "This creature gains hexproof until end of turn. Tap it." (CR 702.11b): `this_gains_then_tap` for hexproof. Dream Trawler
/// prints it.
pub fn this_gains_hexproof_then_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_then_tap(g, item, Keyword::Hexproof)
}

/// "`target` becomes a [creature type] in addition to its other types until end of turn" (CR 205.1b, 611.2a, 613.1d): a layer 4
/// effect from `source` that adds the subtype, read through the words of the source's text (CR 612) so a text-changing effect
/// changes it. Sarkhan, Dragon Ascendant and Enter the Avatar State print it.
pub fn becomes_subtype_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId, subtype: Subtype) {
    let subtype = g.words(source).subtype(subtype);
    g.add_effect(source, target, EffectKind::AddSubtype(subtype), Duration::EndOfTurn);
}

/// "Put `COUNTERS` +1/+1 counters on target creature. It gains [keywords] until end of turn" (CR 122.1, 611.2a, 608.2c): the first
/// target, if it is still a legal target (CR 608.2b), gets the counters and then, as a second instruction, the keywords for the rest of
/// the turn; `KEYWORDS` is a `Keywords` as `u128` (`Keywords::of(&[Keyword::Lifelink]).0`). With no target chosen (an "up to one")
/// nothing happens. Bard the Bowman (lifelink), Thranduil's Company (two counters, vigilance), Tam's Resistance (vigilance) and
/// Fateshaper Aspirant's second mode (vigilance and indestructible) print it.
pub fn plus_one_counters_on_first_target_that_gains_until_end_of_turn<const COUNTERS: i32, const KEYWORDS: u128>(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, COUNTERS);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords(KEYWORDS)), Duration::EndOfTurn);
    Ok(())
}

/// "Put `COUNTERS` +1/+1 counters on [this creature]. It gains [keywords] until end of turn" (CR 122.1, 611.2a, 608.2c): the
/// ability's source, and only while it is still the object that was on the battlefield (CR 400.7); the counters, then as a second
/// instruction the keywords for the rest of the turn. `KEYWORDS` is a `Keywords` as `u128`
/// (`Keywords::of(&[Keyword::DoubleStrike]).0`). Pacesetter Paragon (one counter, double strike) and Rough Rhino Cavalry (two
/// counters, trample) print it.
pub fn plus_one_counters_on_this_that_gains_until_end_of_turn<const COUNTERS: i32, const KEYWORDS: u128>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, COUNTERS);
    g.end_sentence();
    g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords(KEYWORDS)), Duration::EndOfTurn);
    Ok(())
}

/// "`target` gains [a keyword] until end of turn" (CR 611.2a): a layer 6 keyword for the rest of the turn, from the effect's
/// source. Creeping Tar Pit, Immolation Shaman and Kappa Cannoneer print it inside a longer instruction ("can't be blocked
/// this turn", "gains menace"); the `this_gains_*` below are the abilities that only give it.
pub fn gain_until_end_of_turn(g: &mut dyn Game, source: ObjectId, target: ObjectId, keyword: Keyword) {
    g.add_effect(source, target, EffectKind::AddKeywords(Keywords::of(&[keyword])), Duration::EndOfTurn);
}

/// "`source` gains your choice of [keywords] until end of turn" (CR 608.2d, 611.2a): `target` gains the one the controller
/// picks from `keywords`, labelled by `card`'s `labels` in the same order. Hungering Puppetbeast (trample, hexproof or haste)
/// and Wingnut, Bat on the Belfry (flying, menace or haste) print it.
pub fn gain_your_choice_until_end_of_turn(g: &mut dyn Game, item: &StackItem, card: &'static CardDef, target: ObjectId, keywords: &[Keyword]) -> Result<(), Suspend> {
    let card = g.card_id(card);
    let labels = (0..keywords.len() as u8).map(|option| Label::Card(card, option)).collect();
    let keyword = keywords[crate::choose::one_of(g, item.controller, item.object, labels)? as usize];
    gain_until_end_of_turn(g, item.object, target, keyword);
    Ok(())
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

/// "Target creature gains flying until end of turn" (CR 702.9b, 611.2a): the ability's first target. A target no longer legal gains
/// nothing (CR 608.2b). Helitrooper and Stratosoarer print it.
pub fn first_target_gains_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    }
    Ok(())
}

/// "This creature gains first strike until end of turn" (CR 702.7a): `this_gains_until_end_of_turn` for first strike.
pub fn this_gains_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::FirstStrike)
}

/// "This creature gains deathtouch until end of turn" (CR 702.2b): `this_gains_until_end_of_turn` for deathtouch.
pub fn this_gains_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::Deathtouch)
}

/// "This creature gains reach until end of turn" (CR 702.17b): `this_gains_until_end_of_turn` for reach. Death-Hood Cobra
/// ({1}{G}) and Moopsy ({2}) print it.
pub fn this_gains_reach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gains_until_end_of_turn(g, item, Keyword::Reach)
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

/// "Target permanent gains hexproof and indestructible until end of turn" (CR 702.11b, 702.12b, 611.2a), the permanent being the
/// spell or ability's first target; nothing if it is no longer a legal target as it resolves (CR 608.2b). Concerted Care, Defend the
/// Rider, Restoration Magic, Vigorbloom Charm and Plaza of Heroes print it.
pub fn target_gains_hexproof_and_indestructible_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = crate::targets::object(g, item, 0) {
        gain_hexproof_and_indestructible_until_end_of_turn(g, item.object, permanent);
    }
    Ok(())
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
/// one move; nothing if that player is no longer a legal target (CR 608.2b). The pile is linked to the ability's source
/// (CR 607.2a), for a linked ability that reads it ("a creature card exiled with it"); nothing returns it (CR 610.3 does not
/// apply). Kutzil's Flanker and Tormod's Crypt print it, Assimilation Assessor with its linked trigger.
pub fn exile_first_target_players_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        let cards = g.state().zone(Zone::graveyard(player)).to_vec();
        g.exile_with(&cards, g.linked(item), false);
    }
    Ok(())
}

/// "Exile any number of target players' graveyards" (CR 406.2, 608.2b): every card in the graveyard of each of the ability's
/// target players still legal, in one move. With no targets, or none still legal, nothing happens. The Death of Gwen
/// Stacy's chapter III and Stonespeaker Crystal print it.
pub fn exile_target_players_graveyards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = crate::targets::legal_players(g, item)
        .into_iter()
        .flat_map(|player| g.state().zone(Zone::graveyard(player)).to_vec())
        .collect();
    g.exile_all(&cards);
    Ok(())
}

/// "Exile all creatures" (CR 406.2, 608.2f): every creature on the battlefield, whoever controls it, in one move, tokens too.
/// How many were exiled (CR 608.2c: the moves made, those a replacement sent elsewhere included; a merged creature once
/// however many cards it became, CR 730.2, 730.3c), for "X is the number of creatures exiled this way". Sunfall and
/// Overwrite the Multiverse print it.
pub fn exile_all_creatures(g: &mut dyn Game) -> usize {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let exiled = g.exile_all(&creatures);
    g.permanents_left(&exiled).len()
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
    gain_control_untap_haste_for(g, item, Duration::EndOfTurn)
}

/// `gain_control_untap_haste` with the control lasting `duration`: Evil's Thrall's control lasts until the end of its controller's
/// next turn when they control a Villain of greater mana value; the haste lasts until end of turn all the same.
pub fn gain_control_untap_haste_for(g: &mut dyn Game, item: &StackItem, duration: Duration) -> Option<ObjectId> {
    let creature = crate::targets::object(g, item, 0)?;
    gain_control_of_untap_haste(g, item, creature, duration);
    Some(creature)
}

/// `gain_control_untap_haste_for` of a creature the ability names some other way than by targeting it: The Wrath of Khan's
/// delayed ability, about the creature its chapter I dealt damage to (CR 603.7c). A creature that has left the battlefield
/// is a new object none of the three instructions reaches (CR 400.7).
pub fn gain_control_of_untap_haste(g: &mut dyn Game, item: &StackItem, creature: ObjectId, duration: Duration) {
    g.gain_control(item.object, creature, item.controller, duration);
    // Three sentences, each judged by the state triggers before the next (CR 603.8, 608.2c).
    g.end_sentence();
    g.untap(creature);
    g.end_sentence();
    crate::haste_until_end_of_turn(g, item.object, creature);
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
pub fn loses_all_abilities_and_becomes(g: &mut dyn Game, item: &StackItem, slot: usize, color: Color, kind: Subtype, power: i32, toughness: i32) {
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

/// "[Target creature or artifact] loses all abilities and becomes a [color] [creature type] creature with base power and
/// toughness [P]/[T]", for good (CR 611.2a, the rulings: the effect doesn't end at end of turn), for the target in `slot`:
/// continuous effects from a resolved ability (CR 611.2), all at its timestamp (CR 613.7a) - layer 4, its card types replaced
/// by creature (the types it had and the subtypes of those going with them, CR 205.1a) and its creature types replaced by the
/// one named, so an artifact becomes only a creature of that type and a creature is that type alone; layer 5, that color and
/// no other (CR 105.3); layer 6, every ability it has by then lost, and one gained later kept; layer 7b, the base power and
/// toughness. The color and the creature type are words of the ability's text (CR 612), read through its words. With the
/// target gone or no longer a legal target, nothing happens (CR 608.2b). Oko, Thief of Crowns (a green Elk, 3/3) and Lizard,
/// Connors's Curse (a green Lizard, 4/4) print it.
pub fn loses_all_abilities_and_becomes_a_creature_for_good(
    g: &mut dyn Game,
    item: &StackItem,
    slot: usize,
    color: Color,
    kind: Subtype,
    power: i32,
    toughness: i32,
) {
    let Some(target) = crate::targets::object(g, item, slot) else { return };
    let source = item.object;
    let words = g.words(source);
    // One instruction (CR 608.2c): the state triggers see it all done.
    instruction(g, |g| {
        g.add_effect(source, target, EffectKind::LoseAllAbilities, Duration::Indefinite);
        g.add_effect(source, target, EffectKind::SetTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
        g.add_effect(source, target, EffectKind::LoseAllCreatureTypes, Duration::Indefinite);
        g.add_effect(source, target, EffectKind::AddSubtype(words.subtype(kind)), Duration::Indefinite);
        g.add_effect(source, target, EffectKind::SetColors(words.colors(ColorSet::of(color))), Duration::Indefinite);
        g.add_effect(source, target, EffectKind::SetPt(power, toughness), Duration::Indefinite);
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
    (power, toughness): (i32, i32),
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

/// "Put `N` -1/-1 counters on target creature" (CR 122.1a, 608.2b), the first target still legal on resolution; nothing when the
/// slot was left empty ("up to one") or the target is gone. Emptiness (three), Bile-Vial Boggart and Nightmare Sower (one) and
/// Blight Rot (four) print it.
pub fn minus_one_counters_on_target<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::MinusOne, N);
    Ok(())
}

/// "Put `N` charge counters on target [permanent]" (CR 122.1a, 608.2b): the first target, and nothing if none was chosen or
/// it is no longer legal as this resolves. Drill Too Deep (five) and Seven of Nine, Tertiary Adjunct (two) print it.
pub fn charge_counters_on_target<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(object, CounterKind::Charge, N);
    Ok(())
}

/// "Put a +1/+1 counter on target [permanent]" (CR 122.1a, 608.2b), the first target still legal on resolution.
/// Bristly Bill, Spine Sower, Bow of Nylea, Yoshimaru, Beloved Companion, Kenrith, the Returned King and
/// Gallia, the Merrymaker print it.
pub fn plus_one_counter_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(object, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Put a +1/+1 counter on target creature, then double the number of +1/+1 counters on that creature" (CR 122.1, 701.10e):
/// nothing if the target is no longer legal (CR 608.2b). The count to double is read after the counter is on it, so a creature
/// with none ends with two. Growth Curve and Sazh Katzroy print it.
pub fn plus_one_counter_on_target_then_double(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    crate::double::counters(g, &[creature], CounterKind::PlusOne);
    Ok(())
}

/// "Put [its / those] counters on [up to one] target [permanent]": as many of each kind as `from` had as it left, read from its
/// last existence on the battlefield (CR 603.10a, 608.2h); the counters are put on, not moved (the rulings), every kind of
/// them, -1/-1 counters too, together as one event (CR 122.6). Nothing if the target is no longer legal (CR 608.2b) or none was
/// chosen. `put_its_counters_on_target` is the ability's own source; Donatello, Mutant Mechanic's is the artifact that died.
pub fn put_counters_of_on_target(g: &mut dyn Game, item: &StackItem, from: ObjectId) -> Result<(), Suspend> {
    let Some(target) = crate::targets::object(g, item, 0) else { return Ok(()) };
    put_counters_of_on(g, from, target);
    Ok(())
}

/// The counters `from` had as it last existed put on `target`, every kind together as one event (CR 122.6, 603.10a): Donatello,
/// Mutant Mechanic's target and Buzzard-Wasp Colony's own permanent are `target`.
pub fn put_counters_of_on(g: &mut dyn Game, from: ObjectId, target: ObjectId) {
    let counters = g.obj(from).counter_array();
    let puts: Vec<(ObjectId, CounterKind, i32)> = CounterKind::ALL.into_iter().map(|kind| (target, kind, counters[kind as usize])).collect();
    g.add_counters_together(&puts);
}

/// "When this creature dies [or leaves the battlefield], put its counters on [up to one] target creature" (see `put_counters_of_on_target`). Essence
/// Channeler, Graft Surgeon, Scolding Administrator and Broodguard Elite print it.
pub fn put_its_counters_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    put_counters_of_on_target(g, item, item.object)
}

/// "Put three +1/+1 counters on target artifact you control. If it isn't a creature, it becomes a 0/0 Robot creature in
/// addition to its other types" (CR 122.1a, 205.1b, 613.1d): the first target, if still legal. The counters always land; the
/// target is made a creature only if it needed to become one, so an artifact creature keeps the power and toughness it had.
/// Does Machines and Donatello, Mutant Mechanic print it.
pub fn three_counters_and_becomes_robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    three_counters_and_becomes(g, item, Subtype::Robot)
}

/// "Put three +1/+1 counters on [up to one] target [permanent]. If it isn't a creature, it becomes a 0/0 `kind` creature in
/// addition to its other types" (see `three_counters_and_becomes_robot`, which is the Robot of it). Pizza Face, Gastromancer
/// prints the Mutant.
pub fn three_counters_and_becomes(g: &mut dyn Game, item: &StackItem, kind: Subtype) -> Result<(), Suspend> {
    let Some(target) = crate::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(target, CounterKind::PlusOne, 3);
    g.end_sentence();
    if !g.chars(target).is(CardType::Creature) {
        let kind = g.words(item.object).subtype(kind);
        instruction(g, |g| {
            g.add_effect(item.object, target, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
            g.add_effect(item.object, target, EffectKind::AddSubtype(kind), Duration::Indefinite);
            g.add_effect(item.object, target, EffectKind::SetPt(0, 0), Duration::Indefinite);
        });
    }
    Ok(())
}

/// "You may cast [the target card] from your graveyard without paying its mana cost. If that spell would be put into your
/// graveyard, exile it instead." The cast is part of the resolution, ignoring timing (CR 608.2g); an {X} in the cost is 0
/// (CR 107.3b, the rulings). A cast that cannot be completed is given up (CR 733.1). Dreadhorde Arcanist and The Dawning
/// Archaic print it.
pub fn may_cast_first_target_free_exiling_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    may_cast_first_target(g, item, Some(ManaCost::ZERO), false, ExileInstead::Spell)
}

/// "You may cast [the target card] from your graveyard. If that spell would be put into your graveyard, exile it instead":
/// `may_cast_first_target_free_exiling_it` with its costs paid (CR 601.2f-h), its mana cost or an alternative cost it has, and an
/// {X} chosen as it is cast (the rulings). Chandra, Acolyte of Flame's last ability prints it.
pub fn may_cast_first_target_exiling_it_paying_costs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    may_cast_first_target(g, item, None, false, ExileInstead::Spell)
}

/// "You may cast [the target card]" as the ability resolves, whatever the timing restrictions of its types (CR 608.2g): the
/// first target is cast if its controller says yes and it can be cast; the way is chosen for a card that has several (CR 715.3,
/// 720.3), and a cast that cannot be paid for is given up (CR 733.1). `instead` is the cost paid in place of its own (`None`:
/// its own costs), `any_type` is "mana of any type can be spent to cast that spell" (CR 609.4b), and `exile_instead` the card's own
/// "If that spell would be put into a graveyard, exile it instead" (CR 614.1a) or `ExileInstead::Never`. Tinybones, the
/// Pickpocket (`any_type`) and Quistis Trepe (`any_type`, exiling) print it, besides the two wrappers above.
pub fn may_cast_first_target(g: &mut dyn Game, item: &StackItem, instead: Option<ManaCost>, any_type: bool, exile_instead: ExileInstead) -> Result<(), Suspend> {
    let Some(card) = crate::targets::object(g, item, 0) else { return Ok(()) };
    if !g.could_cast_during_resolution(card) || !ask_may(g, item.controller, item.object, Purpose::Cast)? {
        return Ok(());
    }
    let ways = g.cast_ways_during_resolution(card);
    let way = crate::choose::cast_way(g, item.controller, card, ways)?.map(|w| w.way);
    g.cast_during_resolution_as(card, CastTerms { instead, any_type, exile_instead, way, ..CastTerms::default() })?;
    Ok(())
}

/// "[This creature] connives" (CR 701.50): the source of the ability connives, even if it has already left the battlefield. Ledger
/// Shredder, M.O.D.O.K., Madame Masque, Norman Osborn and Red Room Recruit print it.
pub fn this_connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::connive(g, item.object, item.object)
}

/// "Target [creature] connives" (CR 701.50): the first target, if it is still legal (CR 608.2b). Leader, Super-Genius and Villainous
/// Hideout print it.
pub fn target_connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    crate::connive(g, creature, item.object)
}

/// "Return the exiled card to its owner's hand" (CR 607.2a): the cards exiled with this ability's object, linked to it. Tidehollow
/// Sculler's second ability, and Champion of the Clachan's and Champions of the Shoal's, whose cost exiled it (CR 607.2q).
pub fn return_exiled_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    g.return_to_hand(&exiled);
    Ok(())
}

/// "Target player gains N life" (CR 119.3, 608.2b): the first targeted player gains N life if still legal.
/// Healing Salve, Alabaster Potion and Stream of Life print it with different amounts.
pub fn target_player_gains_life(g: &mut dyn Game, item: &StackItem, n: i32) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, 0) {
        g.gain_life(player, n);
    }
    Ok(())
}

/// "Enchanted/equipped creature gets +P/+T until end of turn" (CR 608.2h, 611.2a): use the attachment as the ability
/// resolves, or as the source last existed, and affect only that same object if it remains on the battlefield.
/// Firebreathing and Umezawa's Jitte print this recipe.
pub fn attached_creature_gets<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        crate::pump(g, item.object, creature, P, T);
    }
    Ok(())
}

/// "This creature gets +P/+T until end of turn" (CR 611.2a). Fallen Angel, Astute Terraformer and Super-Skrull print it.
pub fn pump_self<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    this_gets::<P, T>(g, item)
}

/// "Target creature has base power and toughness P/T until end of turn" (CR 613.4b, 611.2a): the first target,
/// if still legal, gets a layer 7b setting effect. Sorceress Queen, Multiply by Zero, Perfected Theory and Quandrix Charm print it.
pub fn set_first_target_base_pt<const P: i32, const T: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = crate::targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::SetPt(P, T), Duration::EndOfTurn);
    }
    Ok(())
}

/// Damage equal to the number of cards in a player's hand (CR 608.2h).
/// Storm Seeker and Emberwilde Captain use this amount.
pub fn damage_for_cards_in_hand(q: &dyn Query, player: PlayerId) -> u32 {
    crate::cards_in_hand(q, player) as u32
}

/// Add N colorless mana (CR 106.4). Su-Chi, Konstrari Charm and Ugin, Eye of the Storms use this effect.
pub fn add_colorless<const N: u32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, N);
    Ok(())
}

/// Switch a creature's power and toughness until end of turn (CR 613.4d, 611.2a).
/// Transmutation and Twisted Image print this effect.
pub fn switch_power_toughness_this_turn(g: &mut dyn Game, source: ObjectId, creature: ObjectId) {
    g.add_effect(source, creature, EffectKind::SwitchPt, Duration::EndOfTurn);
}

/// Triskelion and Walking Ballista both remove a +1/+1 counter to deal one damage to any target (CR 602.2, 115.4).
pub const REMOVE_PLUS_ONE_TO_DEAL_ONE: AbilityDef = ability(
    remove_counters(CounterKind::PlusOne, 1),
    damage_first_target::<1>,
    "Remove a +1/+1 counter from this creature: It deals 1 damage to any target.",
).targets(&[crate::targets::ANY]);

/// "Permanents your opponents control lose hexproof and indestructible until end of turn" (CR 611.2c, 702.11, 702.12): the permanents
/// those opponents control as the ability resolves, and only those, so one that arrives later keeps both. Shadowspear and The Fire
/// Nation Drill print it.
pub fn opponents_permanents_lose_hexproof_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lost = Keywords::of(&[Keyword::Hexproof, Keyword::Indestructible]);
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let permanents: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| opponents.contains(&g.controller(id))).collect();
    instruction(g, |g| {
        for permanent in permanents {
            g.add_effect(item.object, permanent, EffectKind::LoseKeywords(lost), Duration::EndOfTurn);
        }
    });
    Ok(())
}

/// "You may pay [cost]. If you do, return this card from your graveyard to the battlefield [with counters on it]" (CR 118.12,
/// 603.4, 122.6): the payment is chosen as the ability resolves, and the card returns as a new object under its owner's control. Once the
/// card is no longer the object that triggered (CR 400.7) nothing is asked and nothing returns. The card is asked of its own
/// graveyard, so the ability functions only there (CR 113.6m). Forsaken Miner and Wolfbat print it.
pub fn may_pay_to_return_this_to_the_battlefield(
    g: &mut dyn Game,
    item: &StackItem,
    price: Cost,
    question: &'static Question,
    counters: &[(CounterKind, i32)],
) -> Result<(), Suspend> {
    if is_live(g, item.object) && g.may_pay(item.controller, item.object, item.object, price, question)? {
        g.put_onto_battlefield_with_counters(item.object, item.controller, false, counters)?;
    }
    Ok(())
}

/// "You control target opponent during their next turn" (CR 723.1), the player being the ability's first target: its controller
/// makes every choice and decision that player would make during the next turn they actually take; nothing if the target is no
/// longer legal as it resolves (CR 608.2b). Construct a Cosmic Cube and The Dominion Bracelet print it.
pub fn control_target_player_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = crate::targets::player(g, item, 0) else { return Ok(()) };
    g.control_next_turn(item, player);
    Ok(())
}

/// "Draw a card for each artifact you control" (CR 121.2): the artifacts the ability's controller controls as it resolves
/// (CR 608.2h), drawn as one draw of that many. Memnarch, the Warden and Edgar, King of Figaro print it.
pub fn draw_a_card_for_each_artifact_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = crate::artifacts_you_control(g, item.controller) as u32;
    g.draw(item.controller, count);
    Ok(())
}

/// "Attacking creatures get +`power`/+`toughness` until end of turn" (CR 611.2c): the creatures attacking as `source`'s spell
/// or ability resolves, whoever controls them, and none that attack or are put onto the battlefield attacking later. One
/// instruction. Army of Allah, Auron's Inspiration and Morale print it.
pub fn attacking_creatures_get(g: &mut dyn Game, source: ObjectId, power: i32, toughness: i32) {
    let attackers: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.is_attacking(id)).collect();
    instruction(g, |g| {
        for id in attackers {
            crate::pump(g, source, id, power, toughness);
        }
    });
}

/// "Blocking creatures get +`power`/+`toughness` until end of turn" (CR 611.2c): the creatures blocking as `source`'s spell
/// or ability resolves, whoever controls them, and none that block later. One instruction. Piety and Rally print it.
pub fn blocking_creatures_get(g: &mut dyn Game, source: ObjectId, power: i32, toughness: i32) {
    let blockers: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.is_blocking(id)).collect();
    instruction(g, |g| {
        for id in blockers {
            crate::pump(g, source, id, power, toughness);
        }
    });
}

/// "Tap target [permanent]. It doesn't untap during its controller's untap step for as long as this [permanent] remains tapped"
/// (CR 701.26a, 611.2a, 502.3): taps the target if it is still legal (CR 608.2b), and that is a separate sentence from the
/// effect that holds it tapped (CR 608.2c). The card also sets `may_not_untap`. Ice Floe, Mole Worms and Phyrexian Gremlins
/// print it.
pub fn tap_target_it_doesnt_untap_while_this_remains_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = crate::targets::object(g, item, 0) else { return Ok(()) };
    tap_targets(g, item)?;
    g.end_sentence();
    g.add_effect(item.object, target, EffectKind::DoesntUntap, Duration::WhileSourceTapped);
    Ok(())
}

/// "Untap all creatures that attacked this turn" (CR 701.26b, 508.1, 400.7): Own the Day and World at War.
/// Current creatures, irrespective of controller; only the same object that was declared attacking counts.
/// All untap as one instruction, before any resulting triggers or state-based actions are processed.
pub fn untap_creatures_that_attacked_this_turn(g: &mut dyn Game) {
    let creatures: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.history().attacked_this_turn(id)).collect();
    g.untap_together(&creatures);
}

/// "Untap all other creatures you control" (CR 701.26b): Combat Celebrant and Ahn-Crop Champion.
/// The ability's controller chooses the set as it resolves; its source identity stays excluded even if
/// that object left, returned as a new object, or changed controllers (CR 113.7a, 400.7, 608.2h).
pub fn untap_other_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<_> = g.permanents_of(item.controller).into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    g.untap_together(&creatures);
    Ok(())
}

/// "Put N [kind] counters on this permanent" (CR 122.1a), where KIND is CounterKind::X as usize.
/// Armageddon Clock puts a doom counter and Time Bomb a time counter; Weather Maker puts a charge counter.
/// The engine handles a source that has left, placement replacements and counter restrictions.
pub fn counters_on_self<const KIND: usize, const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::ALL[KIND], N);
    Ok(())
}

/// "You gain life equal to the sacrificed creature's toughness" (CR 608.2h, 119.3), as Diamond Valley and Life Chisel
/// print it. The first cost object is its original battlefield identity, so chars reads its last-known toughness.
/// The ability's controller gains the life; a nonpositive toughness does not make that player lose life (CR 107.1b).
pub fn gain_life_equal_sacrificed_creatures_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount = g.cost_objects(item).first().map_or(0, |&object| g.chars(object).toughness.unwrap_or(0)).max(0);
    g.gain_life(item.controller, amount);
    Ok(())
}

/// "Put N charge counters on this artifact" (CR 122.1a): the source itself, as the ability resolves. Weather Maker prints it
/// with one, Starship Enterprise with four.
pub fn charge_counters_on_self<const N: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_self::<{ CounterKind::Charge as usize }, N>(g, item)
}

/// "Draw that many cards" (CR 121.1, 608.2c): as many cards as the combat damage the trigger's event dealt to the player
/// (`StackItem::matched_damage_amount`). April, Reporter of the Weird prints it before a discard, Starship Enterprise alone.
pub fn draw_that_many(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, item.matched_damage_amount());
    Ok(())
}

/// "Draw X cards, then you may put up to X land cards from your hand onto the battlefield tapped" (CR 121.1, 608.2c, 608.2d,
/// 614.1c): the draw is a sentence of its own, so the cards just drawn are in the hand to choose from (CR 603.3b), and the
/// "up to" is answered with any number from none to X. The lands are chosen together and put tapped (CR 101.4), and putting
/// one onto the battlefield is not playing it, so it uses no land play (CR 305.2). Lt. Arex, Ever-Adapting prints it with
/// three, Starfleet Communicator with X.
pub fn draw_then_may_put_up_to_lands_tapped(g: &mut dyn Game, player: PlayerId, source: ObjectId, most: u32) -> Result<(), Suspend> {
    g.draw(player, most);
    g.end_sentence();
    crate::hand::may_put_up_to_onto_battlefield_tapped(g, player, source, crate::select::land_cards(crate::select::up_to(most)))?;
    Ok(())
}
