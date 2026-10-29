use cantrip_api::*;
use cantrip_mechanics::combat::create_attacking_of_your_choice;
use cantrip_mechanics::effects;
use cantrip_mechanics::tokens::REBEL;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// You get an experience counter (CR 122.1). Then create a 2/2 red Rebel creature token that's tapped and attacking (CR 508.4)
/// for each experience counter you have: the player's running total, the new counter counted. The text names nothing for
/// them to attack, so their controller chooses for each one, a player, a planeswalker or a battle, not necessarily what
/// Otharri attacks (CR 508.4); the ones attacking the same thing are created together (CR 101.4).
fn rebels(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.give_experience(you, 1);
    let count = g.state().player(you).experience() as usize;
    let rebel = g.card_id(&REBEL);
    create_attacking_of_your_choice(g, you, item.object, rebel, count, true)?;
    Ok(())
}

fn untapped_rebel(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Rebel))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, rebels, "Whenever Otharri attacks, you get an experience counter. Then create a 2/2 red Rebel creature token that's tapped and attacking for each experience counter you have."),
        ability(cost("{2}{W}{R}").and(tap_untapped(1, untapped_rebel)), effects::return_this_card_to_battlefield_tapped, "{2}{R}{W}, Tap an untapped Rebel you control: Return this card from your graveyard to the battlefield tapped.")
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::OTHARRI_SUNS_GLORY
};

pub static EXTRA: &[&CardDef] = &[&REBEL];
