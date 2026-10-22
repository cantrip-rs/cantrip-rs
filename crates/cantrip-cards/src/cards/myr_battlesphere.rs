use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::pump;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

fn myr(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::MYR, 4)?;
    Ok(())
}

/// You may tap X untapped Myr you control (X chosen as the ability resolves, CR 608.2d). If you do, this creature
/// gets +X/+0 until end of turn and deals X damage to the player or planeswalker it was attacking: a battle is
/// neither (CR 310.9b), and a permanent that has stopped being a planeswalker is no longer "the planeswalker".
fn tap_myr(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let myr = g.words(item.object).subtype(Subtype::Myr);
    let untapped_myr: Vec<ObjectId> = g
        .permanents_of(player)
        .into_iter()
        .filter(|&id| !g.obj(id).tapped && g.chars(id).subtypes.has(myr))
        .collect();
    let chosen = choose::any_number_of(g, player, item.object, Purpose::TapToPay, "", untapped_myr.clone(), untapped_myr)?;
    if chosen.is_empty() {
        return Ok(());
    }
    for &card in &chosen {
        g.tap(card);
    }
    let x = chosen.len() as i16;
    pump(g, item.object, item.object, x, 0);
    let hit = match item.subject {
        Target::Player(p) => Some(Target::Player(p)),
        Target::Object(id)
            if targets::permanent(g, id) && g.chars(id).is(CardType::Planeswalker) =>
        {
            Some(Target::Object(id))
        }
        _ => None,
    };
    if let Some(target) = hit {
        g.damage(item.object, target, x as u16);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, myr, "When this creature enters, create four 1/1 colorless Myr artifact creature tokens."),
        triggered(triggers::THIS_ATTACKS_DEFENDER, tap_myr, "Whenever this creature attacks, you may tap X untapped Myr you control. If you do, this creature gets +X/+0 until end of turn and deals X damage to the player or planeswalker it's attacking."),
    ],
    ..oracle::MYR_BATTLESPHERE
};

pub static EXTRA: &[&CardDef] = &[&tokens::MYR];
