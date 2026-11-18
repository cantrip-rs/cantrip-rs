use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST;
use cantrip_mechanics::{choose, craft, effects, select, triggers};
use cantrip_oracle as oracle;

/// "For each player, choose a creature with power 2 or less that player controls. Then destroy all creatures except creatures
/// chosen this way." (CR 608.2d, 701.8a): the ability's controller makes every choice, in turn order, before anything is
/// destroyed; a player with no such creature has none chosen (the choice is made where it can be). Then every other creature,
/// whoever controls it, is destroyed at once - a creature a player could have chosen but didn't, included.
fn destroy_all_but_a_small_creature_of_each_player(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut chosen: Vec<ObjectId> = Vec::new();
    for who in g.state().apnap().collect::<Vec<_>>() {
        let small: Vec<ObjectId> = g
            .permanents_of(who)
            .into_iter()
            .filter(|&id| {
                let chars = g.chars(id);
                chars.is(CardType::Creature) && chars.power.is_some_and(|power| power <= 2)
            })
            .collect();
        chosen.extend(choose::select(g, you, item.object, Purpose::Keep, small, select::cards(select::exactly(1)))?);
    }
    let doomed: Vec<ObjectId> = effects::creatures_on_the_battlefield(g).into_iter().filter(|id| !chosen.contains(id)).collect();
    g.destroy_all(&doomed);
    Ok(())
}

/// "Whenever an opponent casts a spell during their turn" (CR 603.2, 601.2i): `an_opponent_casts_a_spell`, while it is that
/// opponent's own turn.
fn an_opponent_casts_a_spell_during_their_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::an_opponent_casts_a_spell(q, me, event)?;
    matches!(event, Event::SpellCast { player, .. } if *player == q.state().turn.active).then_some(fired)
}

/// "They can't attack you or planeswalkers you control this turn" (CR 508.1c, 611.2c): a rule about the creatures of whoever's
/// turn it is as the rule is read - the opponent who cast the spell, since it was cast in their own turn and ends with it.
fn forbid_their_attacks_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::Player(item.controller), Duration::EndOfTurn);
    g.end_sentence();
    Ok(())
}

/// The creatures of the player whose turn it is: the opponent who cast the spell.
fn their_creatures(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.controller(id) == q.state().turn.active
}

/// "Each opponent who attacked you or a planeswalker you control this turn can't cast spells" (CR 101.2, 508.1, 601.2a) as a
/// `CardDef::cast_rules` entry: a creature was declared as an attacker this turn by the caster, an opponent of this
/// permanent's controller, with that controller as its defending player (a battle they protect is no attack on them, ruling).
fn opponents_who_attacked_you_cant_cast_spells(q: &dyn Query, asker: &Asker, caster: PlayerId, _from: ZoneKind) -> bool {
    let attacked_you = q.history().this_turn().iter().any(|r| {
        matches!(r.event, Event::Attacks { defender, defending, .. }
            if r.active == caster && defending == asker.controller && !matches!(defender, Target::Object(id) if q.chars(id).is(CardType::Battle)))
    });
    !(attacked_you && q.state().opponents(asker.controller).any(|o| o == caster))
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, an_opponent_casts_a_spell_during_their_turn),
        forbid_their_attacks_this_turn,
        "Whenever an opponent casts a spell during their turn, they can't attack you or planeswalkers you control this turn.",
    )],
    lasting_rules: &[LastingRule {
        text: "They can't attack you or planeswalkers you control this turn.",
        rule: LastingRuleKind::CantAttackYouOrPlaneswalkers,
        applies: their_creatures,
    }],
    cast_rules: &[opponents_who_attacked_you_cant_cast_spells],
    ..oracle::SANDSWIRL_WANDERGLYPH
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_IF_CAST,
            destroy_all_but_a_small_creature_of_each_player,
            "When this artifact enters, if you cast it, for each player, choose a creature with power 2 or less that player controls. Then destroy all creatures except creatures chosen this way.",
        ),
        craft::ability(mana("{3}{W}{W}"), &craft::ARTIFACT, "Craft with artifact {3}{W}{W}"),
    ],
    back: Some(&BACK),
    ..oracle::UNSTABLE_GLYPHBRIDGE
};
