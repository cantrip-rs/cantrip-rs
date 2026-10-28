use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Whenever a creature card leaves your graveyard" (CR 603.10a): a card that was a creature card in the source's
/// controller's graveyard and went anywhere else, however it was moved; each card is its own event, so several leaving
/// together trigger it as many times (CR 603.2c). A token is never a card there, and another player's graveyard is no one's
/// but theirs.
fn creature_card_leaves_your_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. } if *from == Zone::graveyard(me.controller) && q.chars(*old).is(CardType::Creature) => Fired::PLAIN,
        _ => None,
    }
}

/// "Amass Goblins 1." (CR 701.47a.)
fn amass_goblins(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, 1)
}

/// "Goblins and Orcs you control gain menace until end of turn" (CR 611.2c, 702.111): the permanents you control that are
/// Goblins or Orcs as the ability resolves, and none that become one or arrive later. The words are read through this
/// enchantment's text (CR 612.2).
fn goblins_and_orcs_gain_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let goblin = g.words(item.object).subtype(Subtype::Goblin);
    let orc = g.words(item.object).subtype(Subtype::Orc);
    let tribe: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            chars.has_subtype(goblin) || chars.has_subtype(orc)
        })
        .collect();
    instruction(g, |g| {
        for id in tribe {
            effects::gain_until_end_of_turn(g, item.object, id, Keyword::Menace);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this enchantment enters, return target creature card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
        triggered(when_asked(EventMask::ZONE_CHANGE, creature_card_leaves_your_graveyard), amass_goblins, "Whenever a creature card leaves your graveyard, amass Goblins 1."),
        ability(cost("{1}{B}"), goblins_and_orcs_gain_menace, "{1}{B}: Goblins and Orcs you control gain menace until end of turn."),
    ],
    ..oracle::ALONG_THE_CROOKED_WAY
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
