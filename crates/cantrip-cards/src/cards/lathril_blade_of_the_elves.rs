use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_mechanics::{effects, lands, tokens, untapped_subtype_you_control};
use cantrip_oracle as oracle;

/// "Create that many 1/1 green Elf Warrior creature tokens" (CR 111.1, 101.4): as many as the combat damage Lathril dealt to the
/// player (the trigger's `damage_amount`), all made together.
fn create_that_many_elf_warriors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELF_WARRIOR, item.matched_damage_amount())?;
    Ok(())
}

/// Menace comes from the Oracle keywords (CR 702.110). The cost "{T}, Tap ten untapped Elves you control" (CR 602.2b, 118.3) is
/// Lathril's own {T}, so she must have been under her controller's control continuously since their most recent turn began (CR
/// 302.6), and ten other permanents tapped as a cost, which need not have been (the ruling); Lathril, an Elf herself, is not among
/// the ten, being tapped by her {T} - so it takes ten Elves besides her.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
            create_that_many_elf_warriors,
            "Whenever Lathril deals combat damage to a player, create that many 1/1 green Elf Warrior creature tokens.",
        )
        .damage_amount(),
        ability(
            TAP.and(tap_untapped(10, untapped_subtype_you_control::<{ lands::sub(Subtype::Elf) }>)),
            effects::each_opponent_loses_life_you_gain_life::<10>,
            "{T}, Tap ten untapped Elves you control: Each opponent loses 10 life and you gain 10 life.",
        ),
    ],
    ..oracle::LATHRIL_BLADE_OF_THE_ELVES
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF_WARRIOR];
