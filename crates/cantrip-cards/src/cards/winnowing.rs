use cantrip_api::*;
use cantrip_mechanics::{choose, select, statics};
use cantrip_oracle as oracle;

/// "For each player, you choose a creature that player controls. Then each player sacrifices all other creatures they
/// control that don't share a creature type with the chosen creature they control." (CR 608.2d, 701.21a, 101.4): the spell's
/// controller makes every choice, in turn order, before anything is sacrificed; a player who controls no creature has none
/// chosen and sacrifices nothing. Then the creatures other than a player's chosen one that share no creature type with it
/// are sacrificed, all players' together. A changeling shares a type with every creature that has one.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut doomed: Vec<(PlayerId, ObjectId)> = Vec::new();
    for who in g.state().apnap().collect::<Vec<_>>() {
        let creatures: Vec<ObjectId> = g.permanents_of(who).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
        let keep = choose::select(g, you, item.object, Purpose::Keep, creatures.clone(), select::cards(select::exactly(1)))?;
        let Some(&keep) = keep.first() else { continue };
        let types = g.chars(keep).subtypes;
        doomed.extend(creatures.into_iter().filter(|&id| id != keep && !statics::shares_creature_type(types, g.chars(id).subtypes)).map(|id| (who, id)));
    }
    g.sacrifice_each(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::WINNOWING };
