use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Draw two cards. Create a tapped Planet land token named New Planet" (CR 121.2, 111.1). The land's
/// sacrifice and tap are activation costs; draw happens before the token is created.
fn draw_and_create(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    tokens::create_by_rule(g, item.controller, &tokens::NEW_PLANET, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{5}{U}").and(TAP).and(SACRIFICE_THIS), draw_and_create, "{5}{U}, {T}, Sacrifice this land: Draw two cards. Create a tapped Planet land token named New Planet with \"{T}: Add one mana of any color.\""),
    ],
    ..oracle::GASEOUS_ANOMALY
};

pub static EXTRA: &[&CardDef] = &[&tokens::NEW_PLANET];
