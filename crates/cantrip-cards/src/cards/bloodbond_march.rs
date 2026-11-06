use cantrip_api::*;
use cantrip_mechanics::same_name;
use cantrip_oracle as oracle;

/// "Whenever a player casts a creature spell": any player's, read as it stands on the stack. The spell rides along as
/// the trigger's subject ("that spell").
fn creature_spell_cast(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    cantrip_mechanics::triggers::player_casts_spell_of_type(q, event, CardType::Creature)
}

/// Each player returns all cards with the same name as that spell from their graveyard to the battlefield, in one
/// event, each under its owner's control (CR 110.2a). The name is the spell's as the ability resolves, or as it last
/// was if it has left the stack (CR 608.2h).
fn return_same_name(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let mut returning: Vec<(ObjectId, PlayerId)> = Vec::new();
    for player in g.state().seats().collect::<Vec<_>>() {
        for &card in g.state().zone(Zone::graveyard(player)) {
            if same_name(g, spell, card) {
                returning.push((card, player));
            }
        }
    }
    if !returning.is_empty() {
        g.put_all_onto_battlefield_by(&returning.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::SPELL_CAST, creature_spell_cast), return_same_name, "Whenever a player casts a creature spell, each player returns all cards with the same name as that spell from their graveyard to the battlefield.")],
    ..oracle::BLOODBOND_MARCH
};
