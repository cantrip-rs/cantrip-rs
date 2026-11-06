use cantrip_api::*;
use cantrip_oracle as oracle;

/// Deal damage simultaneously to every other player, then gain the damage actually dealt (CR 120.3, 119.3).
fn syphon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<_> = g.state().apnap().filter(|&p| p != item.controller)
        .map(|p| (item.object, Target::Player(p), 2u32)).collect();
    let dealt = g.damage_together(&hits).into_iter().fold(0u32, u32::saturating_add);
    g.end_sentence();
    g.gain_life(item.controller, dealt.min(i32::MAX as u32) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], syphon),
    ..oracle::SYPHON_SOUL
};
