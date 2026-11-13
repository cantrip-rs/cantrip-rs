use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Each player mills X cards. For each creature card put into a graveyard this way, you create a tapped 2/2 black Zombie creature
/// token." (CR 701.17a, 101.4, 111.1): every player mills X together; `mill_together` returns the cards that went to a graveyard or,
/// by a replacement, to exile instead (CR 701.17c), so only those now in a graveyard and creature cards count, whoever's they are.
/// The tokens are created by the spell's controller, all tapped, together (CR 101.4).
fn each_player_mills_x_then_a_tapped_zombie_per_creature_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mills: Vec<(PlayerId, u32)> = g.state().apnap().map(|p| (p, item.x as u32)).collect();
    let milled = g.mill_together(&mills);
    let creature_cards = milled.iter().filter(|&&c| g.obj(c).zone.kind() == ZoneKind::Graveyard && g.chars(c).is(CardType::Creature)).count();
    let zombie = g.card_id(&tokens::ZOMBIE);
    g.create_tokens(zombie, item.controller, creature_cards as u32, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], each_player_mills_x_then_a_tapped_zombie_per_creature_card),
    ..oracle::DREAD_SUMMONS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];
