use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    g.each_opponent_loses_life(player, 2);
    g.gain_life(player, 2);
    tokens::create_by_rule(g, player, &tokens::NEW_PLANET, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{3}{B}").and(TAP).and(SACRIFICE_THIS), drain, "{3}{B}, {T}, Sacrifice this land: Each opponent loses 2 life and you gain 2 life. Create a tapped Planet land token named New Planet with \"{T}: Add one mana of any color.\""),
    ],
    ..oracle::ARTIFICIAL_ANOMALY
};

pub static EXTRA: &[&CardDef] = &[&tokens::NEW_PLANET];
