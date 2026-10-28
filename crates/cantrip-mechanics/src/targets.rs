//! Target slots many cards print (CR 115).

use cantrip_api::*;

fn on_battlefield(q: &dyn Query, id: ObjectId) -> bool {
    q.is_on_battlefield(id) && q.obj(id).became.is_none()
}

fn any_target_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    match t {
        Target::Player(_) => true,
        Target::Object(id) => creature_planeswalker_or_battle(q, id),
        Target::None | Target::Ability(_) => false,
    }
}

/// A creature, planeswalker or battle on the battlefield (CR 110.4).
fn creature_planeswalker_or_battle(q: &dyn Query, id: ObjectId) -> bool {
    on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) || c.is(CardType::Planeswalker) || c.is(CardType::Battle)
    }
}

fn creature_planeswalker_or_battle_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if creature_planeswalker_or_battle(q, id))
}

/// "Target creature, planeswalker, or battle" (CR 115.1, 110.4). Volcanic Spite, Cosmic Hunger, Nahiri's Warcrafting, Shatter
/// the Source and Tandem Takedown print it.
pub const CREATURE_PLANESWALKER_OR_BATTLE: TargetSpec = target(creature_planeswalker_or_battle_filter);

fn creature_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && q.chars(id).is(CardType::Creature))
}

fn tapped_creature_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).tapped)
}

fn creature_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

fn creature_you_dont_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

fn player_or_planeswalker_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    match t {
        Target::Player(_) => true,
        Target::Object(id) => on_battlefield(q, id) && q.chars(id).is(CardType::Planeswalker),
        Target::None | Target::Ability(_) => false,
    }
}

fn creature_or_planeswalker_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) || c.is(CardType::Planeswalker)
    })
}

fn player_filter(_: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(_))
}

fn opponent_filter(_: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(p) if p != asker.controller)
}

fn spell_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id))
}

/// Any target (CR 115.4): a creature, a player, a planeswalker or a battle.
pub const ANY: TargetSpec = target(any_target_filter);
pub const CREATURE: TargetSpec = target(creature_filter);

fn nonlegendary_creature_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).supertypes.legendary)
}

/// "Target nonlegendary creature" (CR 115.1, 205.4a): a creature on the battlefield without the legendary supertype.
/// Mirrorweave and Yoshimaru, Scrappy Stray print it; checked again as the ability resolves (CR 608.2b).
pub const NONLEGENDARY_CREATURE: TargetSpec = target(nonlegendary_creature_filter);

fn legendary_creature_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).supertypes.legendary)
}

/// "Target legendary creature" (CR 115.1, 205.4a), whoever controls it. Karakas and Yoshimaru, Beloved Companion print it.
pub const LEGENDARY_CREATURE: TargetSpec = target(legendary_creature_filter);

fn creature_with_toughness_four_or_greater_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).toughness.is_some_and(|v| v >= 4))
}
/// "Target creature with toughness 4 or greater" (CR 115.1) as the layers give the toughness now, so a counter or an
/// effect moves a creature in and out of range. Valorous Stance, Surgical Precision and Destroy Evil print it.
pub const CREATURE_WITH_TOUGHNESS_4_OR_GREATER: TargetSpec = target(creature_with_toughness_four_or_greater_filter);
/// "A creature with flying" (CR 702.9) as the layers give it now: a creature on the battlefield that has flying. Pick Your Poison
/// sacrifices one; `CREATURE_WITH_FLYING` is the target slot. Bow of Nylea and Konstrari Charm print the target.
pub fn has_flying_creature(q: &dyn Query, id: ObjectId) -> bool {
    let c = q.chars(id);
    c.is(CardType::Creature) && c.has(Keyword::Flying)
}
/// "Another target creature", "other target creature" (CR 115.1, 109.5): a creature on the battlefield other than the source
/// of the spell or ability. Arwen, Mortal Queen, Restless Vinestalk, Wolverine, Fierce Fighter, Solitude, Abigale, Eloquent First-Year, Lyna, Veil of Vengeance
/// and Super-Adaptoid print it; a slot of up to one
/// is `up_to_one(other_creature_filter)`.
pub fn other_creature_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && t != Target::Object(asker.source)
}
pub const OTHER_CREATURE: TargetSpec = target(other_creature_filter);
/// "Target tapped creature" (CR 115.1, 608.2b): tapped when chosen and, legality being checked again on resolution, still
/// tapped then. Ajani, Outland Chaperone, Cryogen Relic and The Wandering Emperor print it.
pub const TAPPED_CREATURE: TargetSpec = target(tapped_creature_filter);
fn tapped_creature_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    tapped_creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}
/// "Target tapped creature an opponent controls" (CR 115.1, 608.2b). Spring-Loaded Sawblades and Super Villain Lockup print it.
pub const TAPPED_CREATURE_AN_OPPONENT_CONTROLS: TargetSpec = target(tapped_creature_an_opponent_controls_filter);
fn attacking_creature_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.is_attacking(id))
}

/// "Target attacking creature" (CR 115.1, 506.4): a creature on the battlefield that is attacking when it is chosen and
/// still is as the ability resolves, or the target is illegal. Raffine, Scheming Seer and Raubahn, Bull of Ala Mhigo print it.
pub const ATTACKING_CREATURE: TargetSpec = target(attacking_creature_filter);

/// "Target attacking or blocking creature" (CR 506.4, 508.1, 509.1): a creature on the battlefield that is attacking or
/// blocking now. Gideon's Memorial and Eiganjo, Seat of the Empire print it.
pub fn attacking_or_blocking_creature_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if permanent(q, id)
        && q.chars(id).is(CardType::Creature)
        && (q.is_attacking(id) || q.state().blocks.iter().any(|b| b.blocker == id)))
}
pub const ATTACKING_OR_BLOCKING_CREATURE: TargetSpec = target(attacking_or_blocking_creature_filter);
pub const CREATURE_YOU_CONTROL: TargetSpec = target(creature_you_control_filter);

/// "A creature is attacking alone if it's attacking but no other creatures are" (CR 506.5): attacking now, whether declared as an
/// attacker or put onto the battlefield attacking, and the only attacking creature.
pub fn attacking_alone(q: &dyn Query, id: ObjectId) -> bool {
    q.is_attacking(id) && q.state().attackers.len() == 1
}
fn creature_attacking_alone_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if attacking_alone(q, id))
}
/// "Target creature that's attacking alone" (CR 115.1, 506.5): judged as it is chosen and again as the ability resolves. Viper,
/// Cruel Conspirator prints it.
pub const CREATURE_ATTACKING_ALONE: TargetSpec = target(creature_attacking_alone_filter);
fn creature_you_control_attacking_alone_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_attacking_alone_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}
/// "Target creature you control that's attacking alone" (CR 115.1, 506.5). Crowd of True Believers prints it.
pub const CREATURE_YOU_CONTROL_ATTACKING_ALONE: TargetSpec = target(creature_you_control_attacking_alone_filter);
/// "Another target creature you control", "other target creature you control" (CR 115.1, 109.5): a creature on the battlefield
/// under your control other than the source. Dour Port Mage, Giver of Runes, Oliphaunt, Flock Impostor and the reconfigure
/// ability print it; a slot of up to one is `up_to_one(other_creature_you_control_filter)`.
pub fn other_creature_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_you_control_filter(q, asker, t) && t != Target::Object(asker.source)
}
pub const OTHER_CREATURE_YOU_CONTROL: TargetSpec = target(other_creature_you_control_filter);

/// "Creature or Vehicle" (CR 205.3g, 301.7): a permanent on the battlefield that is a creature, or a Vehicle whether or not
/// it is crewed into a creature right now. Bounce Off and Perilous Snare target one; Reckoner Shakedown chooses one.
pub fn creature_or_vehicle(q: &dyn Query, id: ObjectId) -> bool {
    on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) || c.has_subtype(Subtype::Vehicle)
    }
}

fn creature_or_vehicle_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if creature_or_vehicle(q, id))
}

fn creature_or_vehicle_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if creature_or_vehicle(q, id) && q.controller(id) == asker.controller)
}

/// "Target creature or Vehicle" (CR 115.1). Bounce Off prints it.
pub const CREATURE_OR_VEHICLE: TargetSpec = target(creature_or_vehicle_filter);
/// "Target creature or Vehicle you control" (CR 115.1, 109.5). Perilous Snare prints it.
pub const CREATURE_OR_VEHICLE_YOU_CONTROL: TargetSpec = target(creature_or_vehicle_you_control_filter);

/// "Creature or Spacecraft" (CR 205.3g), as Beyond the Quiet's "all creatures and Spacecraft" reads it too: a permanent
/// on the battlefield that is a creature, or a Spacecraft whether or not it is a creature right now (it is one only while
/// stationed).
pub fn creature_or_spacecraft(q: &dyn Query, id: ObjectId) -> bool {
    on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) || c.has_subtype(Subtype::Spacecraft)
    }
}

fn creature_or_spacecraft_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if creature_or_spacecraft(q, id))
}

/// "Target creature or Spacecraft" (CR 115.1). 70,000 Light-Years from Home, Gravkill and Embrace Oblivion print it.
pub const CREATURE_OR_SPACECRAFT: TargetSpec = target(creature_or_spacecraft_filter);
/// "Target creature you don't control" (CR 109.5): any other player's creature, a teammate's included. Winds of Abandon and
/// Bushwhack print it.
pub const CREATURE_YOU_DONT_CONTROL: TargetSpec = target(creature_you_dont_control_filter);
/// "Target player or planeswalker" (CR 115.1, 115.4): any target narrowed to players and planeswalkers. Boros Charm, Kyren
/// Negotiations and Quicksilver Dagger print it.
pub const PLAYER_OR_PLANESWALKER: TargetSpec = target(player_or_planeswalker_filter);
/// "Target creature or planeswalker" (CR 115.1). Unholy Heat and Baleful Mastery print it.
pub const CREATURE_OR_PLANESWALKER: TargetSpec = target(creature_or_planeswalker_filter);
pub const PLAYER: TargetSpec = target(player_filter);
pub const OPPONENT: TargetSpec = target(opponent_filter);
pub const SPELL: TargetSpec = target(spell_filter);
fn spell_or_ability_with_a_single_target_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let stack = &q.state().stack;
    match t {
        Target::Object(id) => stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id && i.target_count == 1),
        Target::Ability(serial) => stack.iter().any(|i| i.kind != StackKind::Spell && i.serial == serial && i.target_count == 1),
        _ => false,
    }
}

fn spell_or_ability_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let stack = &q.state().stack;
    match t {
        Target::Object(id) => stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id),
        Target::Ability(serial) => stack.iter().any(|i| i.kind != StackKind::Spell && i.serial == serial),
        _ => false,
    }
}

/// "Target spell or ability" and "target spell, activated ability, or triggered ability" (CR 115.1): a spell or an ability on
/// the stack (a mana ability never is, CR 605.3b). Spellskite and Ertai Resurrected print it.
pub const SPELL_OR_ABILITY: TargetSpec = target(spell_or_ability_filter);

/// "Target activated or triggered ability" (CR 115.1): an ability on the stack, which a mana ability never is (CR 605.3b).
/// Stifle ("target") and Tishana's Tidebinder ("up to one target") print it.
pub fn activated_or_triggered_ability_filter(_: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Ability(_))
}
pub const ACTIVATED_OR_TRIGGERED_ABILITY: TargetSpec = target(activated_or_triggered_ability_filter);

/// "Target spell or ability with a single target" (CR 115.1, 115.7a): exactly one target across every slot, counted with
/// repeats, as the item was announced; re-asked as the spell resolves (CR 608.2b). Untimely Malfunction and Bolt Bend print it.
pub const SPELL_OR_ABILITY_WITH_A_SINGLE_TARGET: TargetSpec = target(spell_or_ability_with_a_single_target_filter);
/// "Target spell or permanent" (CR 115.1): Artificial Evolution and Venser, Shaper Savant print it.
pub const SPELL_OR_PERMANENT: TargetSpec = target(spell_or_permanent_filter);

fn spell_or_permanent_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    spell_filter(q, asker, t) || permanent_filter(q, asker, t)
}

/// Whether a target is a card in a graveyard with one of these types.
pub fn card_in_graveyard(q: &dyn Query, t: Target, types: &[CardType]) -> bool {
    match t {
        Target::Object(id) => {
            let obj = q.obj(id);
            obj.zone.is(ZoneKind::Graveyard) && obj.became.is_none() && types.iter().any(|&ty| q.chars(id).is(ty))
        }
        _ => false,
    }
}

fn instant_or_sorcery_in_your_graveyard_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_graveyard(q, t, &[CardType::Instant, CardType::Sorcery])
        && matches!(t, Target::Object(id) if q.obj(id).zone == Zone::graveyard(asker.controller))
}

/// "Target instant or sorcery card in your graveyard" (CR 115.1). Snapcaster Mage and Flashback print it.
pub const INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD: TargetSpec =
    target(instant_or_sorcery_in_your_graveyard_filter);

fn noncreature_artifact_or_enchantment_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    match t {
        Target::Object(id) if q.is_on_battlefield(id) => {
            let c = q.chars(id);
            !c.is(CardType::Creature) && (c.is(CardType::Artifact) || c.is(CardType::Enchantment))
        }
        _ => false,
    }
}

/// "Target noncreature artifact or noncreature enchantment" (CR 115.1). Guerrilla Gorilla and Haywire Mite print it.
pub const NONCREATURE_ARTIFACT_OR_ENCHANTMENT: TargetSpec =
    target(noncreature_artifact_or_enchantment_filter);

/// "... that player controls" in a slot of "for each opponent, up to one target [permanent] that player controls" (CR 601.2c, 115.1): a
/// permanent on the battlefield controlled by the opponent whose slot (`TargetSpec::EACH_OPPONENT`) is being chosen or checked, which
/// the core says through `Query::slot_opponent`. A slot's filter adds the kind of permanent. The Theorist, Jace Beleren's minus two and
/// Lich's Relic's reflexive destroy print it.
pub fn controlled_by_slot_opponent(q: &dyn Query, id: ObjectId) -> bool {
    permanent(q, id) && Some(q.controller(id)) == q.slot_opponent()
}

/// "With mana value X" in a target filter (CR 107.3a, 202.3): `id`'s mana value is the X announced for the spell or ability
/// whose target is judged (`Query::announced_x`): its own X, as it is announced, as it is judged again on resolution
/// (CR 608.2b) and as its targets change (CR 115.7), never another activation's of the same source (CR 107.3k). Before X
/// is announced, when it is only asked whether some announcement could have a legal target, anything is, since some X would
/// allow it; so the slot is chosen after X (`late_targets`, `targets_after_x`, CR 601.2b-c). An object with {X} in its
/// mana cost has X of 0 off the stack, which `Query::mana_value` gives. Repeal, Likeness Looter and The Mycosynth Gardens
/// print it.
pub fn mana_value_is_x(q: &dyn Query, id: ObjectId) -> bool {
    q.announced_x().is_none_or(|x| q.mana_value(id) == x)
}

/// The object a resolving item targeted in `slot`, if the target is still legal (CR 608.2b) and is an object (not a
/// player). Every effect that acts on "target [thing]" begins here rather than pairing `target_legal` with a pattern
/// match. Used by `destroy_target`, `exile_target` and by most of the cards that act on one target.
pub fn object(g: &dyn Game, item: &StackItem, slot: usize) -> Option<ObjectId> {
    if slot < item.target_count as usize && g.target_legal(item, slot) { g.state().target(item, slot).object() } else { None }
}

/// The player a resolving item targeted in `slot`, if the target is still legal (CR 608.2b) and is a player: `object`'s twin
/// for "target player". Used by `effects::target_player_draws`.
pub fn player(g: &dyn Game, item: &StackItem, slot: usize) -> Option<PlayerId> {
    if slot < item.target_count as usize && g.target_legal(item, slot) { g.state().target(item, slot).player() } else { None }
}

/// Every player target of the item that is still legal (CR 608.2b), in the order they were chosen: "any number of target
/// players each ..." resolves for the ones still legal. Riverchurn Monument and Tinybones Joins Up print it.
pub fn legal_players(g: &dyn Game, item: &StackItem) -> Vec<PlayerId> {
    (0..item.target_count as usize).filter_map(|slot| player(g, item, slot)).collect()
}

/// Every object target of the item that is still legal (CR 608.2b), in slot order. Fiery Confluence-style spells and
/// Echoing Truth-style "each target" effects read it.
pub fn legal_objects(g: &dyn Game, item: &StackItem) -> Vec<ObjectId> {
    (0..item.target_count as usize).filter_map(|slot| object(g, item, slot)).collect()
}

fn equipment_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if permanent(q, id) && q.controller(id) == asker.controller && q.chars(id).has_subtype(Subtype::Equipment))
}
/// "Target Equipment you control" (CR 115.1, 301.5). Super-Soldier Serum, Raubahn and Thorin print it.
pub const EQUIPMENT_YOU_CONTROL: TargetSpec = target(equipment_you_control_filter);

/// A permanent that is still the object it was on the battlefield (CR 400.7): the start of every battlefield filter.
pub fn permanent(q: &dyn Query, id: ObjectId) -> bool {
    on_battlefield(q, id)
}

/// "[This permanent]'s power" read by an ability of it: the power it has now, or the power it last had on the battlefield
/// once it has left (CR 608.2h, 113.7a); zero for one with none. Ouroboroid's "where X is Ouroboroid's power" and Loki
/// Laufeyson's "less than or equal to Loki's power" read it.
pub fn power_or_last_known(q: &dyn Query, id: ObjectId) -> i16 {
    if permanent(q, id) {
        q.chars(id).power.unwrap_or(0)
    } else {
        q.state().last_known(id).and_then(|k| k.power).unwrap_or(0)
    }
}

fn artifact_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && q.chars(id).is(CardType::Artifact))
}

fn creature_with_flying_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if has_flying_creature(q, id))
}

/// "Target creature with flying" (CR 115.1, 702.9), read from its computed characteristics. Bow of Nylea, Pawpatch
/// Formation, Glorious Decay, Warg Tactics and Konstrari Charm print it.
pub const CREATURE_WITH_FLYING: TargetSpec = target(creature_with_flying_filter);

/// "Target creature or enchantment" (CR 115.1): the filter itself, for a card that adds to it ("an opponent controls").
/// Shredder's Technique prints it as it is and Feed the Swarm adds the opponent.
pub fn creature_or_enchantment(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Creature) || c.is(CardType::Enchantment)
    })
}

fn artifact_or_enchantment_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Enchantment)
    })
}

fn artifact_or_enchantment_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    artifact_or_enchantment_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "A token you control" as an object filter (a cost's "sacrifice a token", CR 701.21): a token its controller controls,
/// whatever it is. Fountainport sacrifices one.
pub fn token_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.obj(id).token && q.controller(id) == asker.controller
}

fn token_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && token_you_control(q, asker, id))
}

/// "Target token you control" (CR 115.1). Esika's Chariot prints it.
pub const TOKEN_YOU_CONTROL: TargetSpec = target(token_you_control_filter);

fn permanent_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id))
}

/// "Target permanent you control" (CR 115.1, 110.2): a permanent on the battlefield that the ability's or spell's controller
/// controls, judged again as it resolves (CR 608.2b), so one whose control changed in response is no longer a legal target.
pub fn permanent_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

fn nonland_permanent_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && !q.chars(id).is(CardType::Land))
}

/// "Target nonland permanent you control" as a filter (CR 115.1), for a card that narrows it further or asks for it
/// as "up to one". Displacer Kitten and Nurturing Pixie print it.
pub fn nonland_permanent_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// "Other target nonland permanent" as a filter (CR 115.1): a nonland permanent that is not the source of the ability, for a
/// slot of "up to one" or "up to two". Aang's Iceberg, Phelia, Exuberant Shepherd and Marang River Regent print it.
pub fn other_nonland_permanent_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_filter(q, asker, t) && t != Target::Object(asker.source)
}

fn nonland_permanent_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "Target nonland permanent an opponent controls with mana value `N` or less" (CR 115.1, 202.3: X on the battlefield is 0).
/// Portable Hole and Seam Rip (2) and Thundering Broodwagon (4) print it.
pub fn nonland_permanent_an_opponent_controls_with_mana_value_at_most<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_an_opponent_controls_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= N)
}

fn creature_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

fn land_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && q.chars(id).is(CardType::Land))
}

/// "Target creature or land" (CR 115.1, 205.2): a permanent on the battlefield that is a creature or a land as the layers have
/// it now (a land that is a creature too is both). Quake, Agent of S.H.I.E.L.D., Fissure, Lava Flow, Wrecking Ball, Minion of
/// Leshrac and Civic Gardener print it.
pub fn creature_or_land_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && { let c = q.chars(id); c.is(CardType::Creature) || c.is(CardType::Land) })
}
pub const CREATURE_OR_LAND: TargetSpec = target(creature_or_land_filter);

/// The filter of `NONBASIC_LAND`, for a card that narrows it further.
pub fn nonbasic_land_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    land_filter(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).supertypes.basic)
}

fn nonbasic_land_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonbasic_land_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

fn land_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    land_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

fn card_in_a_graveyard_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if { let o = q.obj(id); o.zone.is(ZoneKind::Graveyard) && o.became.is_none() })
}

fn creature_spell_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    spell_filter(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature))
}

/// "Target creature spell" (CR 115.1, 111.1): a spell on the stack with the card type creature, whatever else it is (an
/// artifact creature, an enchantment creature). Essence Scatter prints it; Stern Scolding's "creature spell with power or
/// toughness 2 or less" starts from it.
pub const CREATURE_SPELL: TargetSpec = target(creature_spell_filter);

fn noncreature_spell_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    spell_filter(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).is(CardType::Creature))
}

fn artifact_or_creature_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && q.controller(id) == asker.controller && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Creature)
    })
}

/// "Target artifact" (CR 115.1). Abrade and Embereth Shieldbreaker print it.
pub const ARTIFACT: TargetSpec = target(artifact_filter);

fn enchantment_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && q.chars(id).is(CardType::Enchantment))
}

/// "Target enchantment" (CR 115.1). Requisition Raid, Heritage Reclamation, Pawpatch Formation and Insidious Fungus print it.
pub const ENCHANTMENT: TargetSpec = target(enchantment_filter);
/// "Target artifact or creature you control" (CR 115.1, 205.2a). Concerted Care, Oko, Thief of Crowns (the permanent it exchanges),
/// Ozolith, the Shattered Spire and Three Steps Ahead print it.
pub const ARTIFACT_OR_CREATURE_YOU_CONTROL: TargetSpec = target(artifact_or_creature_you_control_filter);
/// "Target artifact or creature": a permanent that is either (CR 115.1). Oko, Thief of Crowns, Tezzeret, Cruel Captain, Touch
/// the Spirit Realm, Iceberg Titan and Lodestone Needle print it.
pub const ARTIFACT_OR_CREATURE: TargetSpec = target(artifact_or_creature_filter);
/// The filter of `ARTIFACT_OR_CREATURE`, for a slot that is "up to one" of them.
pub fn artifact_or_creature_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Creature)
    })
}
/// "Target artifact or enchantment". Disenchant and Cathar Commando print it.
pub const ARTIFACT_OR_ENCHANTMENT: TargetSpec = target(artifact_or_enchantment_filter);
/// "Target artifact, creature, or enchantment": a permanent that is any of them (CR 115.1). March of Otherworldly Light (with
/// a mana value limit of its own) and Teferi, Time Raveler (up to one) print it.
pub fn artifact_creature_or_enchantment_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if on_battlefield(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Creature) || c.is(CardType::Enchantment)
    })
}
/// "Target permanent". Alpha Deathclaw and Argentum Armor print it.
pub const PERMANENT: TargetSpec = target(permanent_filter);
/// "Target permanent you control". Harmless Offering and Deceiver Exarch print it.
pub const PERMANENT_YOU_CONTROL: TargetSpec = target(permanent_you_control_filter);
/// "Target nonland permanent". Banishing Knack and Boomerang Basics print it.
pub const NONLAND_PERMANENT: TargetSpec = target(nonland_permanent_filter);

/// "Target nonland permanent with mana value `N` or less" (CR 115.1, 202.3: X in a mana cost is 0 on the battlefield), whoever
/// controls it. Abrupt Decay (3), Vraska, Golgari Queen (3) and Witherbloom Charm (2) print it.
pub fn nonland_permanent_with_mana_value_at_most<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= N)
}

/// "Target spell with mana value `N` or less" (CR 115.1, 202.3): a spell on the stack, its mana value asked of the spell as it
/// is (an {X} counts as the value chosen for it, CR 202.3e; a face-down spell is 0, CR 708.2), as it is cast and again as it
/// resolves (CR 608.2b). Change the Equation (2) and Spell Queller (4) print it.
pub fn spell_with_mana_value_at_most<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    spell_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= N)
}

/// "Target creature with mana value `N` or less" (CR 115.1, 202.3: X in a mana cost is 0 on the battlefield), whoever
/// controls it. Requiting Hex (2) and Claim the Firstborn (3) print it.
pub fn creature_with_mana_value_at_most<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= N)
}

/// "Target noncreature, nonland permanent" (CR 115.1): a permanent that is neither, so no creature that is also a land.
/// Archive Arbiter and Witherbloom Command (with a mana value limit) print it.
pub fn noncreature_nonland_permanent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    nonland_permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).is(CardType::Creature))
}

/// "Target noncreature, nonland permanent with mana value `N` or less" (CR 115.1, 202.3: X in a mana cost is 0 on the
/// battlefield), whoever controls it. Witherbloom Command (2) and Molten Collapse (1) print it.
pub fn noncreature_nonland_permanent_with_mana_value_at_most<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    noncreature_nonland_permanent(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= N)
}

/// "Target permanent with mana value `N` or greater" (CR 115.1, 202.3: X in a mana cost is 0 on the battlefield, a copy has
/// its copiable mana value, a face-down permanent 0): any permanent, land or not, whoever controls it. Despark (4) and
/// Kin-Tree Severance (3) print it.
pub fn permanent_with_mana_value_at_least<const N: u32>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) >= N)
}

/// "Target artifact or enchantment an opponent controls". Cathartic Parting, Witch Enchanter, Wistfulness and Scrapshooter print it.
pub const ARTIFACT_OR_ENCHANTMENT_AN_OPPONENT_CONTROLS: TargetSpec = target(artifact_or_enchantment_an_opponent_controls_filter);
fn permanent_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    permanent_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "Target permanent an opponent controls" (CR 115.1). Assassin's Trophy and the emblem of Teferi, Hero of Dominaria print it.
pub const PERMANENT_AN_OPPONENT_CONTROLS: TargetSpec = target(permanent_an_opponent_controls_filter);
/// "Target nonland permanent an opponent controls". Conclave Tribunal and Sheltered by Ghosts print it.
pub const NONLAND_PERMANENT_AN_OPPONENT_CONTROLS: TargetSpec =
    target(nonland_permanent_an_opponent_controls_filter);
/// "Target nonland permanent an opponent controls with mana value 2 or less" (CR 115.1, 202.3: X on the battlefield is
/// 0). Portable Hole and Seam Rip print it.
pub const NONLAND_PERMANENT_AN_OPPONENT_CONTROLS_WITH_MANA_VALUE_2_OR_LESS: TargetSpec =
    target(nonland_permanent_an_opponent_controls_with_mana_value_at_most::<2>);
fn creature_or_planeswalker_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_or_planeswalker_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "Target creature or planeswalker an opponent controls" (CR 115.1), worded "you don't control" by Hard-Hitting Question. A
/// creature or a planeswalker under any other player's control. Compel Brutality and Hard-Hitting Question print it.
pub const CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS: TargetSpec = target(creature_or_planeswalker_an_opponent_controls_filter);
/// "Target creature an opponent controls". Palace Jailer and Mawloc print it.
pub const CREATURE_AN_OPPONENT_CONTROLS: TargetSpec = target(creature_an_opponent_controls_filter);
/// "Target land". Candelabra of Tawnos and Strip Mine print it.
pub const LAND: TargetSpec = target(land_filter);
/// "Target nonbasic land" (CR 115.1, 205.4a). Wasteland prints it.
pub const NONBASIC_LAND: TargetSpec = target(nonbasic_land_filter);
/// "Target nonbasic land an opponent controls" (CR 115.1, 205.4a). Demolition Field and Magmatic Hellkite print it.
pub const NONBASIC_LAND_AN_OPPONENT_CONTROLS: TargetSpec = target(nonbasic_land_an_opponent_controls_filter);
/// "Target land you control". Earthbend (Badgermole Cub) and Sea Drake print it.
pub const LAND_YOU_CONTROL: TargetSpec = target(land_you_control_filter);
/// "Target card from your graveyard" (CR 115.1): any card in the graveyard of the ability's controller, still the card that
/// was there (CR 400.7). Eternal Witness, Emeritus of Abundance, Bala Ged Recovery, Cathartic Parting, Tamiyo, Collector of Tales
/// and Quintorius Kand ("any number of target cards from your graveyard") print it.
pub fn any_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[])
}

/// "Target card from your graveyard": see `any_card_in_your_graveyard`.
pub const CARD_IN_YOUR_GRAVEYARD: TargetSpec = target(any_card_in_your_graveyard);

fn creature_card_in_a_graveyard_filter(q: &dyn Query, _: &Asker, t: Target) -> bool {
    card_in_graveyard(q, t, &[CardType::Creature])
}

/// "Target creature card from a graveyard" (CR 115.1): a creature card in any player's graveyard. Reanimate, Necromancy, From
/// the Catacombs, Portal to Phyrexia and Virtue of Persistence print it.
pub const CREATURE_CARD_IN_A_GRAVEYARD: TargetSpec = target(creature_card_in_a_graveyard_filter);

fn creature_put_onto_the_battlefield_with_this(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).entered_with == Some(asker.source))
}

/// "Enchant creature put onto the battlefield with [this]" (CR 303.4a, 704.5m): legal exactly for the creature that entered
/// with this permanent as its Aura (`Game::put_onto_battlefield_with_aura`). The enchant ability Animate Dead gains and the
/// one Necromancy becomes an Aura with (`CardDef::becomes_aura_enchant`).
pub const CREATURE_PUT_ONTO_THE_BATTLEFIELD_WITH_THIS: TargetSpec = target(creature_put_onto_the_battlefield_with_this);

/// The second slot of "up to two target cards from a single graveyard" (CR 115.1, 601.2c): a card of the graveyard the
/// first slot's card is in, and not that card itself. When the first slot chose nothing, any card in any graveyard: one
/// target is a legal choice of "up to two", and the two slots together allow exactly the choices of zero, one or two
/// cards that all lie in one graveyard. Cease // Desist and Unlicensed Hearse print it.
pub fn another_card_in_the_same_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    if !card_in_a_graveyard_filter(q, asker, t) {
        return false;
    }
    match q.chosen_targets().first() {
        Some(&Target::Object(first)) => first != id && q.obj(first).zone == q.obj(id).zone,
        _ => true,
    }
}

/// The second slot of "up to two target cards from a single graveyard": see `another_card_in_the_same_graveyard`.
pub const ANOTHER_CARD_IN_THE_SAME_GRAVEYARD: TargetSpec = target(another_card_in_the_same_graveyard);

/// "Target card from a graveyard". Emperor of Bones, Ghost Vacuum, Keen-Eyed Curator and Lion Sash print it.
pub const CARD_IN_A_GRAVEYARD: TargetSpec = target(card_in_a_graveyard_filter);
/// "Target card from an opponent's graveyard" (CR 115.1, 404.1): a card in the graveyard of a player other than the ability's
/// controller, still that card (CR 400.7). Gollum, the Abandoned and Lord Skitter, Sewer King ("up to one") print it.
pub fn card_in_an_opponents_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if {
        let o = q.obj(id);
        o.zone.is(ZoneKind::Graveyard) && o.zone.player != asker.controller && o.became.is_none()
    })
}

/// "Target card from an opponent's graveyard": see `card_in_an_opponents_graveyard`.
pub const CARD_IN_AN_OPPONENTS_GRAVEYARD: TargetSpec = target(card_in_an_opponents_graveyard);

/// "Target card from defending player's graveyard" (CR 115.1, 508.5): any card in the graveyard of the defending player of
/// the ability's source - the player it attacks, or, once it has left combat, the one it was attacking
/// (`Query::defending_player`) - still that card (CR 400.7). Hive of the Eye Tyrant and Graven Abomination print it.
pub fn card_in_defending_players_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if {
        let o = q.obj(id);
        o.zone.is(ZoneKind::Graveyard) && o.became.is_none() && q.defending_player(asker.source) == Some(o.zone.player)
    })
}

/// "Target card from defending player's graveyard": see `card_in_defending_players_graveyard`.
pub const CARD_IN_DEFENDING_PLAYERS_GRAVEYARD: TargetSpec = target(card_in_defending_players_graveyard);

fn instant_or_sorcery_spell_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    spell_filter(q, asker, t) && matches!(t, Target::Object(id) if { let c = q.chars(id); c.is(CardType::Instant) || c.is(CardType::Sorcery) })
}

fn instant_or_sorcery_spell_an_opponent_controls_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    instant_or_sorcery_spell_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

fn instant_or_sorcery_spell_you_control_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    instant_or_sorcery_spell_filter(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// "Target instant or sorcery spell" (CR 115.1, 304.1, 307.1): a spell on the stack, not a permanent spell, with the card
/// type instant or sorcery. Test of Talents prints it.
pub const INSTANT_OR_SORCERY_SPELL: TargetSpec = target(instant_or_sorcery_spell_filter);

/// "Target instant or sorcery spell an opponent controls" (CR 115.1, 109.5). Venser, Fervent Forger prints it.
pub const INSTANT_OR_SORCERY_SPELL_AN_OPPONENT_CONTROLS: TargetSpec = target(instant_or_sorcery_spell_an_opponent_controls_filter);

/// "Target instant or sorcery spell you control" (CR 115.1, 109.5). Lutri, the Spellchaser prints it.
pub const INSTANT_OR_SORCERY_SPELL_YOU_CONTROL: TargetSpec = target(instant_or_sorcery_spell_you_control_filter);

/// "Target noncreature spell". Spell Pierce, Stubborn Denial, Negate and Dovin's Veto print it.
pub const NONCREATURE_SPELL: TargetSpec = target(noncreature_spell_filter);

/// "Target [color A] or [color B] spell" (CR 115.1, 105.2): a spell on the stack that is either color, or both. The color
/// words are those of the targeting spell's own text (CR 612), so a text-changing effect on it changes them; `A` and `B`
/// are `Color`s as `u8`. Flashfreeze (red or green) and Precise Redaction (white or black) print it.
fn spell_of_either_color_filter<const A: u8, const B: u8>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let words = q.words(asker.source);
    spell_filter(q, asker, t)
        && matches!(t, Target::Object(id) if {
            let colors = q.chars(id).colors;
            colors.has(words.color(Color::ALL[A as usize])) || colors.has(words.color(Color::ALL[B as usize]))
        })
}

/// "Target [color A] or [color B] spell": see `spell_of_either_color_filter`. Used as
/// `targets::spell_of_either_color::<{ Color::Red as u8 }, { Color::Green as u8 }>()`.
pub const fn spell_of_either_color<const A: u8, const B: u8>() -> TargetSpec {
    target(spell_of_either_color_filter::<A, B>)
}

/// "Target creature or planeswalker that's [color A] or [color B]" (CR 115.1, 105.2): a creature or planeswalker that is
/// either color, or both. The color words are those of the targeting spell's own text (CR 612); `A` and `B` are `Color`s as
/// `u8`. Devout Decree (black or red), Refute Destiny (green or blue), Terminal Criticism (blue or red) and, worded
/// "target black or green creature or planeswalker", Essence Burn print it.
fn creature_or_planeswalker_of_either_color_filter<const A: u8, const B: u8>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    if !(CREATURE_OR_PLANESWALKER.filter)(q, asker, t) {
        return false;
    }
    let Target::Object(id) = t else { return false };
    let words = q.words(asker.source);
    let colors = q.chars(id).colors;
    colors.has(words.color(Color::ALL[A as usize])) || colors.has(words.color(Color::ALL[B as usize]))
}

/// "Target creature or planeswalker that's [color A] or [color B]": see `creature_or_planeswalker_of_either_color_filter`.
/// Used as `targets::creature_or_planeswalker_of_either_color::<{ Color::Black as u8 }, { Color::Red as u8 }>()`.
pub const fn creature_or_planeswalker_of_either_color<const A: u8, const B: u8>() -> TargetSpec {
    target(creature_or_planeswalker_of_either_color_filter::<A, B>)
}

/// "Target [color] creature" (CR 115.1, 105.2): a creature of that color, alone or among others. The color word is that of
/// the targeting spell's own text (CR 612); `C` is a `Color` as `u8`. Dark Betrayal prints it; Deathmark and Rending Volley
/// print two colors, built on it.
fn creature_of_color_filter<const C: u8>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    if !(CREATURE.filter)(q, asker, t) {
        return false;
    }
    let Target::Object(id) = t else { return false };
    q.chars(id).colors.has(q.words(asker.source).color(Color::ALL[C as usize]))
}

/// "Target [color] creature": see `creature_of_color_filter`. Used as
/// `targets::creature_of_color::<{ Color::Black as u8 }>()`.
pub const fn creature_of_color<const C: u8>() -> TargetSpec {
    target(creature_of_color_filter::<C>)
}

/// "Target [color A] or [color B] creature" (CR 115.1, 105.2): a creature that is either color, or both (two
/// `creature_of_color_filter`s). Deathmark (green or white) and Rending Volley (white or blue) print it.
fn creature_of_either_color_filter<const A: u8, const B: u8>(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_of_color_filter::<A>(q, asker, t) || creature_of_color_filter::<B>(q, asker, t)
}

/// "Target [color A] or [color B] creature": see `creature_of_either_color_filter`. Used as
/// `targets::creature_of_either_color::<{ Color::Green as u8 }, { Color::White as u8 }>()`.
pub const fn creature_of_either_color<const A: u8, const B: u8>() -> TargetSpec {
    target(creature_of_either_color_filter::<A, B>)
}

/// "This spell costs {N} less to cast if it targets a [color] [creature / spell]" (CR 601.2c, 601.2f): judged once the target is
/// announced, as the total cost is determined; generic mana only (CR 118.7a). The color word is that of the casting spell's own
/// text (CR 612); `C` is a `Color` as `u8` (a value past the five colors does not compile). It checks the target's color only: its
/// object type (creature, spell) comes from the card's own target spec. The target's color is its color as it is then (on the stack, for a spell). Mystical
/// Dispute (a blue spell) and Knockout Blow (a red creature) print it.
pub fn less_if_it_targets_color<const N: u8, const C: u8>(q: &dyn Query, source: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    const { assert!(C < 5) };
    let color = q.words(source).color(Color::ALL[C as usize]);
    if targets.iter().filter_map(|t| t.object()).any(|object| q.chars(object).colors.has(color)) {
        CostDelta::less(N)
    } else {
        CostDelta::NONE
    }
}

/// Whether a target is a card of one of these types in the asker's own graveyard ("target creature card in your
/// graveyard", CR 115.1). Eternal Witness takes any card (`types` empty); Recurring Nightmare and Metamorphosis Fanatic
/// take a creature card.
pub fn card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target, types: &[CardType]) -> bool {
    match t {
        Target::Object(id) => {
            let obj = q.obj(id);
            obj.zone == Zone::graveyard(asker.controller)
                && obj.became.is_none()
                && (types.is_empty() || types.iter().any(|&ty| q.chars(id).is(ty)))
        }
        _ => false,
    }
}

/// `card_in_your_graveyard` with "mana value N or less" (CR 202.3, `Query::mana_value`; X is 0 in a graveyard,
/// CR 202.3e). Unearth, Carnage, Crimson Chaos, Sevinne's Reclamation and Helping Hand print it.
pub fn card_in_your_graveyard_with_mana_value_at_most(q: &dyn Query, asker: &Asker, t: Target, types: &[CardType], max: u32) -> bool {
    card_in_your_graveyard(q, asker, t, types) && matches!(t, Target::Object(id) if q.mana_value(id) <= max)
}

fn creature_card_with_mana_value_3_or_less_in_your_graveyard_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Creature], 3)
}

/// "Target creature card with mana value 3 or less from your graveyard". Unearth, Helping Hand, Carnage, Crimson Chaos,
/// Call a Surprise Witness and Leonardo's Technique print it.
pub const CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD: TargetSpec = target(creature_card_with_mana_value_3_or_less_in_your_graveyard_filter);

/// The card types of a permanent card (CR 110.4a): those that can exist on the battlefield.
pub const PERMANENT_CARD_TYPES: [CardType; 6] =
    [CardType::Artifact, CardType::Battle, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker];

/// Whether these are the characteristics of a permanent card (CR 110.4a). Malevolent Rumble, Moonshadow, Lion Sash,
/// Faith's Reward and The Aesir Escape Valhalla ask it.
pub fn is_permanent_card(c: &Chars) -> bool {
    PERMANENT_CARD_TYPES.iter().any(|&t| c.is(t))
}

/// Whether these are the characteristics of a nonland permanent card (CR 110.4a): an artifact land is a land. Six,
/// Guardian Scalelord and Loamcrafter Faun ask it.
pub fn is_nonland_permanent_card(c: &Chars) -> bool {
    !c.is(CardType::Land) && is_permanent_card(c)
}

/// "Target permanent card in your graveyard" (CR 115.1, 110.4a). Shifting Woodland prints it; Night Nurse asks it of a
/// card put there this turn.
pub fn permanent_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &PERMANENT_CARD_TYPES)
}

/// "Target permanent card in your graveyard". Shifting Woodland prints it.
pub const PERMANENT_CARD_IN_YOUR_GRAVEYARD: TargetSpec = target(permanent_card_in_your_graveyard);

/// "Target nonland permanent card from your graveyard" (CR 115.1, 110.4a). Loamcrafter Faun prints it; Guardian
/// Scalelord asks it of a card with mana value up to its power.
pub fn nonland_permanent_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if card_in_your_graveyard(q, asker, t, &[]) && is_nonland_permanent_card(&q.chars(id)))
}

/// "Target nonland permanent card from your graveyard". Loamcrafter Faun prints it.
pub const NONLAND_PERMANENT_CARD_IN_YOUR_GRAVEYARD: TargetSpec = target(nonland_permanent_card_in_your_graveyard);

/// "Target land card from your graveyard" (CR 115.1, 400.7). Titania, Protector of Argoth, Wrenn and Six (up to one) and
/// Carnivorous Cultivator print it.
pub fn land_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Land])
}

/// "Target land card from your graveyard": see `land_card_in_your_graveyard`.
pub const LAND_CARD_IN_YOUR_GRAVEYARD: TargetSpec = target(land_card_in_your_graveyard);

fn creature_card_in_your_graveyard_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_in_your_graveyard(q, asker, t, &[CardType::Creature])
}

/// "Target creature card in your graveyard". Recurring Nightmare and Metamorphosis Fanatic print it.
pub const CREATURE_CARD_IN_YOUR_GRAVEYARD: TargetSpec = target(creature_card_in_your_graveyard_filter);

/// "A nontoken artifact" (CR 111.1): an artifact permanent that is a card, not a token. Thopter Foundry sacrifices one as a
/// cost (`filters::nontoken_artifact`) and Lorehold Charm makes each opponent sacrifice one.
pub fn is_nontoken_artifact(q: &dyn Query, id: ObjectId) -> bool {
    !q.obj(id).token && q.chars(id).is(CardType::Artifact)
}

/// A card chosen to be sacrificed, discarded or tapped as a cost: the filter shapes `Cost::sacrifice` and
/// `Cost::discard` take. Each is named by what it accepts.
pub mod filters {
    use cantrip_api::*;

    /// A creature. Bankrupt in Blood and Culling the Weak sacrifice one.
    pub fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).is(CardType::Creature)
    }
    /// Any permanent: Fear of Isolation returns one. Never a discard cost's filter: a filter there names a characteristic of the
    /// cards (CR 701.9c), and "discard a card" is `discard_cards`.
    pub fn any(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
        true
    }
    /// A land. Crop Rotation and Baloth Prime sacrifice one.
    pub fn land(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).is(CardType::Land)
    }
    /// "Another creature" (CR 109.1, 701.21a): a creature other than the source of the cost. Fiend Artisan and Blood Host sacrifice one.
    pub fn another_creature(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
        id != asker.source && creature(q, asker, id)
    }
    /// An artifact. Crabomination (emerge) and Time Sieve sacrifice one.
    pub fn artifact(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).is(CardType::Artifact)
    }
    /// "Another artifact" (CR 109.1, 701.21a): an artifact other than the source of the cost. The permanents a cost can
    /// sacrifice are the paying player's own, so no controller check is needed. Hungering Puppetbeast, Legion Extruder and
    /// Repurposing Bay sacrifice one.
    pub fn another_artifact(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
        id != asker.source && artifact(q, asker, id)
    }
    /// A nontoken artifact. Thopter Foundry sacrifices one.
    pub fn nontoken_artifact(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        super::is_nontoken_artifact(q, id)
    }
    /// A Treasure (CR 111.10a): Magda, Brazen Outlaw's and Magda, the Hoardmaster's "Sacrifice five/three Treasures".
    pub fn treasure(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).has_subtype(Subtype::Treasure)
    }
    /// A Food (CR 111.10b): Cauldron Familiar's "Sacrifice a Food" and Ygra, Eater of All's ward.
    pub fn food(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        q.chars(id).has_subtype(Subtype::Food)
    }
    /// A nonland permanent. Bolas's Citadel sacrifices ten; Rottenmouth Viper any number.
    pub fn nonland(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        !q.chars(id).is(CardType::Land)
    }
    /// An artifact or a creature. Deadly Dispute, Eviscerator's Insight and Fanatical Offering sacrifice one.
    pub fn artifact_or_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Creature)
    }
    /// "Another creature or artifact": an artifact or a creature other than the source of the cost. Broadside Bombardiers and
    /// Umbral Collar Zealot sacrifice one.
    pub fn another_creature_or_artifact(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
        id != asker.source && artifact_or_creature(q, asker, id)
    }
    /// "Another creature or planeswalker" (CR 701.21a): Loot, the Anomaly and Edgar, Ancient Bloodlord sacrifice one.
    pub fn another_creature_or_planeswalker(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
        id != asker.source && {
            let c = q.chars(id);
            c.is(CardType::Creature) || c.is(CardType::Planeswalker)
        }
    }
}
