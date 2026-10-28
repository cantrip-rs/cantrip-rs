use cantrip_api::*;
use cantrip_mechanics::effects::target_player_loses_1_life_you_gain_1;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_DIES, CREATURE_DIES, THIS_ATTACKS, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::{choose, effects, select, targets, tokens};
use cantrip_oracle as oracle;

/// The creatures other than Sephiroth that `player` controls.
fn other_creatures(g: &dyn Game, player: PlayerId, sephiroth: ObjectId) -> Vec<ObjectId> {
    g.permanents_of(player).into_iter().filter(|&id| id != sephiroth && g.chars(id).is(CardType::Creature)).collect()
}

/// "You may sacrifice another creature. If you do, draw a card." (CR 701.21, 608.2d): declining is the empty answer to an
/// up-to-one choice, nobody is asked with no other creature, and the card is drawn only for a creature actually sacrificed.
fn sacrifice_another_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if !effects::may_sacrifice(g, you, item.object, select::another_creature_permanents(select::up_to(1)))?.is_empty() {
        g.draw(you, 1);
    }
    Ok(())
}

/// "Target opponent loses 1 life and you gain 1 life. If this is the fourth time this ability has resolved this turn,
/// transform Sephiroth." (CR 608.2m, 701.27f.) The three earlier resolutions are in this turn's history; the transformation
/// does nothing if Sephiroth has left or already transformed since the ability went on the stack.
fn drain_and_maybe_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let earlier = g.history().resolutions(item.object, item.ability);
    target_player_loses_1_life_you_gain_1(g, item)?;
    if earlier == 3 {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// "Whenever a creature dies, target opponent loses 1 life and you gain 1 life." A leaves-the-battlefield ability, so it
/// looks back (CR 603.10a): the dying creature as it last was, and whatever stopped abilities triggering then (Hushbringer
/// dying in the same wipe). The emblem functions from the command zone, which it never leaves (CR 114.4).
pub static EMBLEM: CardDef = CardDef {
    name: "Sephiroth, One-Winged Angel emblem",
    has_cost: false,
    abilities: &[triggered(
        CREATURE_DIES,
        target_player_loses_1_life_you_gain_1,
        "Whenever a creature dies, target opponent loses 1 life and you gain 1 life.",
    )
    .targets(&[targets::OPPONENT])],
    text: "Whenever a creature dies, target opponent loses 1 life and you gain 1 life.",
    ..CardDef::BLANK
};

/// "Super Nova — As this creature transforms into Sephiroth, One-Winged Angel, you get an emblem" (CR 712.20): applied by the
/// core as the permanent turns over, to whoever controls it then.
fn super_nova(g: &mut dyn Game, asker: &Asker) -> Result<(), Suspend> {
    tokens::create_emblem(g, asker.controller, &EMBLEM);
    Ok(())
}

/// "Whenever Sephiroth attacks, you may sacrifice any number of other creatures. If you do, draw that many cards." The cards
/// drawn are the creatures actually sacrificed (CR 608.2c).
fn sacrifice_any_number_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let eligible = other_creatures(g, you, item.object);
    let chosen = choose::any_number_of(g, you, item.object, Purpose::Sacrifice, "", eligible.clone(), eligible)?;
    let sacrificed = g.sacrifice(you, &chosen).len() as u32;
    if sacrificed > 0 {
        g.draw(you, sacrificed);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    as_transforms: Some(super_nova),
    abilities: &[triggered(
        THIS_ATTACKS,
        sacrifice_any_number_to_draw,
        "Whenever Sephiroth attacks, you may sacrifice any number of other creatures. If you do, draw that many cards.",
    )],
    ..oracle::SEPHIROTH_ONE_WINGED_ANGEL
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS_OR_ATTACKS, sacrifice_another_to_draw, "Whenever Sephiroth enters or attacks, you may sacrifice another creature. If you do, draw a card."),
        triggered(
            ANOTHER_CREATURE_DIES,
            drain_and_maybe_transform,
            "Whenever another creature dies, target opponent loses 1 life and you gain 1 life. If this is the fourth time this ability has resolved this turn, transform Sephiroth.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    back: Some(&BACK),
    ..oracle::SEPHIROTH_FABLED_SOLDIER
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
