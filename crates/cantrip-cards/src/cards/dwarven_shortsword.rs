use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this Equipment enters, create a 2/2 red Dwarf creature token, then attach this Equipment to it" (CR 701.3a): the
/// Equipment is attached only if it is still on the battlefield as this resolves, to the token if it is still there (CR
/// 701.3b).
fn create_dwarf_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::DWARF, 1)?;
    g.end_sentence();
    if let Some(&dwarf) = made.first() {
        g.attach(item.object, dwarf);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, create_dwarf_and_attach, "When this Equipment enters, create a 2/2 red Dwarf creature token, then attach this Equipment to it."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 2>, "Equipped creature gets +1/+2.")],
    ..oracle::DWARVEN_SHORTSWORD
};

pub static EXTRA: &[&CardDef] = &[&tokens::DWARF];
