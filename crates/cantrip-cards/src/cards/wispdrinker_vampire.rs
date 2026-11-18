use cantrip_api::*;
use cantrip_mechanics::triggers::another_creature_you_control_with_power_between_enters;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Creatures you control with power 2 or less gain deathtouch and lifelink until end of turn" (CR 611.2c, 702.2, 702.15): the
/// creatures of the controller with power 2 or less, as modified by everything in effect, as the ability resolves. A creature that
/// enters later, or whose power drops later, is not affected, and one that grows afterwards keeps both abilities.
fn grant_deathtouch_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_where_gain(g, item, &[Keyword::Deathtouch, Keyword::Lifelink], |g, id| g.chars(id).power.is_some_and(|p| p <= 2));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_creature_you_control_with_power_between_enters::<{ i32::MIN }, 2>),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever another creature you control with power 2 or less enters, each opponent loses 1 life and you gain 1 life.",
        ),
        ability(
            cost("{5}{W}{B}"),
            grant_deathtouch_and_lifelink,
            "{5}{W}{B}: Creatures you control with power 2 or less gain deathtouch and lifelink until end of turn.",
        ),
    ],
    ..oracle::WISPDRINKER_VAMPIRE
};
