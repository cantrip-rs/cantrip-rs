namespace Cantrip.Cards.Model;

/// <summary>Card types (CR 205.2a). Listed in full so layer 4 has a closed vocabulary to work with.</summary>
public enum CardType
{
    Artifact = 0,
    Battle = 1,
    Creature = 2,
    Enchantment = 3,
    Instant = 4,
    Kindred = 5,
    Land = 6,
    Planeswalker = 7,
    Sorcery = 8,
}

/// <summary>Supertypes (CR 205.4a).</summary>
public enum Supertype
{
    Basic = 0,
    Legendary = 1,
    Ongoing = 2,
    Snow = 3,
    World = 4,
}

/// <summary>
/// The printed type line (CR 205).
/// <para>
/// Subtypes are strings rather than an enum because the creature-type list is open and grows with
/// every set; the engine only needs to recognise a handful of them specially (the basic land
/// types, for their intrinsic mana ability), and <see cref="BasicLandType"/> does that.
/// </para>
/// </summary>
public sealed class TypeLine
{
    public List<Supertype> Supertypes = [];
    public List<CardType> CardTypes = [];
    public List<string> Subtypes = [];

    public bool Is(CardType type) => CardTypes.Contains(type);

    public bool Has(Supertype supertype) => Supertypes.Contains(supertype);

    public bool HasSubtype(string subtype)
    {
        foreach (string existing in Subtypes)
        {
            if (string.Equals(existing, subtype, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    public override string ToString()
    {
        IEnumerable<string> left = Supertypes.Select(s => s.ToString()).Concat(CardTypes.Select(t => t.ToString()));
        string head = string.Join(' ', left);
        return Subtypes.Count == 0 ? head : $"{head} — {string.Join(' ', Subtypes)}";
    }
}

/// <summary>
/// The basic land types and the mana they produce.
/// <para>
/// Modelled as a property of the <em>subtype</em>, not as a printed ability on the card, because
/// that is what CR 305.6 says: any land with the Forest subtype has "{T}: Add {G}", whether it was
/// printed that way or acquired the subtype from an effect. Getting this right now is what makes
/// "nonbasic lands are Mountains" work later without special-casing.
/// </para>
/// </summary>
/// <summary>
/// The creature types a card filter may name (CR 205.3m), as Ajani, Nacatl Pariah's "other Cats you control". A closed
/// list rather than any word, so a misspelled type is refused at load instead of matching nothing; a type joins it with
/// the first card that asks for one.
/// </summary>
public static class CreatureType
{
    public static readonly IReadOnlyList<string> Known = ["Cat", "Dragon", "Dwarf", "Pilot", "Servo", "Thopter", "Wolf"];

    /// <summary>
    /// Every creature type there is (CR 205.3m), alphabetically, for a card that says "choose a creature type": the
    /// choice is any of these, not only one a filter may name. Time Lord is the one two-word type.
    /// </summary>
    public static readonly IReadOnlyList<string> All =
    [
        "Advisor", "Aenar", "Aetherborn", "Alien", "Ally", "Andorian", "Android", "Angel", "Antelope", "Ape", "Archer", "Archon", "Armadillo", "Army",
        "Artificer", "Assassin", "Assembly-Worker", "Astartes", "Atog", "Aurochs", "Avatar", "Azra", "Badger",
        "Bajoran", "Balloon", "Barbarian", "Bard", "Basilisk", "Bat", "Bear", "Beast", "Beaver", "Beeble", "Beholder",
        "Berserker", "Betazoid", "Bird", "Bison", "Blinkmoth", "Boar", "Bolian", "Borg", "Bringer", "Brushwagg",
        "Caitian", "Camarid", "Camel", "Capybara", "Cardassian", "Caribou", "Carrier", "Cat", "Centaur", "Child", "Chimera", "Citizen", "Cleric", "Clown", "Cockatrice",
        "Construct", "Coward", "Coyote", "Crab", "Crocodile", "Custodes", "Cyberman", "Cyclops", "C’tan", "Dalek",
        "Dauthi", "Demigod", "Demon", "Denobulan", "Deserter", "Detective", "Devil", "Dinosaur", "Djinn", "Doctor", "Dog", "Dragon",
        "Drake", "Dreadnought", "Drix", "Drone", "Druid", "Dryad", "Dwarf", "Echidna", "Efreet", "Egg", "El-Aurian", "Elder",
        "Eldrazi", "Elemental", "Elephant", "Elf", "Elk", "Employee", "Eternal", "Excalbian", "Eye", "Faerie", "Ferengi", "Ferret", "Fish",
        "Flagbearer", "Fox", "Fractal", "Frog", "Fungus", "Gamer", "Gamma", "Gargoyle", "Germ", "Giant", "Giraffe",
        "Gith", "Glimmer", "Gnoll", "Gnome", "Goat", "Goblin", "God", "Golem", "Gorgon", "Gorn", "Graveborn", "Gremlin",
        "Griffin", "Guest", "Hag", "Halfling", "Hamster", "Harpy", "Hedgehog", "Hellion", "Hero", "Hippo",
        "Hippogriff", "Hirogen", "Homarid", "Homunculus", "Horror", "Horse", "Human", "Hydra", "Hyena", "Illusion", "Illyrian", "Imp",
        "Incarnation", "Inhuman", "Inkling", "Inquisitor", "Insect", "Jackal", "Jellyfish", "Juggernaut", "Kangaroo",
        "Kavu", "Kelpien", "Kirin", "Kithkin", "Klingon", "Knight", "Kobold", "Kor", "Kraken", "Kree", "Kwejian", "Lamia", "Lammasu", "Lanthanite", "Leech", "Lemur",
        "Leviathan", "Lhurgoyf", "Licid", "Lizard", "Llama", "Lobster", "Manticore", "Masticore", "Medusan", "Mercenary",
        "Merfolk", "Metathran", "Minion", "Minotaur", "Mite", "Mole", "Monger", "Mongoose", "Monk", "Monkey", "Moogle",
        "Moonfolk", "Mount", "Mouse", "Mutant", "Myr", "Mystic", "Na'kuhl", "Nautilus", "Necron", "Nephilim", "Nightmare",
        "Nightstalker", "Ninja", "Noble", "Noggle", "Nomad", "Nymph", "Ocampa", "Octopus", "Officer", "Ogre", "Ooze", "Orb", "Orc", "Orgg", "Orion",
        "Otter", "Ouphe", "Ox", "Oyster", "Pangolin", "Peasant", "Pegasus", "Pentavite", "Performer", "Pest",
        "Phelddagrif", "Phoenix", "Phyrexian", "Pilot", "Pincher", "Pirate", "Plant", "Platypus", "Porcupine",
        "Possum", "Praetor", "Primarch", "Prism", "Processor", "Q", "Qu", "Rabbit", "Raccoon", "Ranger", "Rat", "Rebel",
        "Reflection", "Rhino", "Rigger", "Robot", "Rogue", "Romulan", "Sable", "Salamander", "Samurai", "Sand", "Saproling",
        "Satyr", "Scarecrow", "Scientist", "Scion", "Scorpion", "Scout", "Sculpture", "Seal", "Serf", "Serpent",
        "Servo", "Shade", "Shaman", "Shapeshifter", "Shark", "Sheep", "Shi’ar", "Siren", "Skeleton", "Skrull", "Skunk",
        "Slith", "Sliver", "Sloth", "Slug", "Snail", "Snake", "Soldier", "Soltari", "Sorcerer", "Spawn", "Specter",
        "Spellshaper", "Sphinx", "Spider", "Spike", "Spirit", "Splinter", "Sponge", "Spy", "Squid", "Squirrel",
        "Starfish", "Surrakar", "Survivor", "Symbiote", "Synth", "Talosian", "Tamarian", "Tellarite", "Tentacle", "Tetravite", "Thalakos", "Tholian", "Thopter",
        "Thrull", "Tiefling", "Time Lord", "Tosk", "Toy", "Treefolk", "Tribble", "Trill", "Trilobite", "Triskelavite", "Troll", "Turtle",
        "Tyranid", "Unicorn", "Utrom", "Vampire", "Varmint", "Vau-N'Akat", "Vedalken", "Villain", "Volver", "Vorta", "Vulcan", "Wall", "Walrus",
        "Warlock", "Warrior", "Weasel", "Weird", "Werewolf", "Whale", "Wizard", "Wolf", "Wolverine", "Wombat", "Worm",
        "Wraith", "Wurm", "Xindi", "Yeti", "Zombie", "Zubera"
    ];
}

/// <summary>
/// The artifact types a card filter may name (CR 205.3g): the predefined tokens', as "Blood tokens you control", plus
/// any real permanent subtype a card asks about, as "search your library for an Equipment card" does. Closed for the
/// same reason <see cref="CreatureType.Known"/> is.
/// </summary>
public static class ArtifactType
{
    public static readonly IReadOnlyList<string> Known = ["Blood", "Clue", "Equipment", "Food", "Treasure"];
}

/// <summary>
/// The enchantment types a card filter may name (CR 205.3h), as "search your library for an Aura or Equipment card"
/// does. Closed for the same reason <see cref="CreatureType.Known"/> is.
/// </summary>
public static class EnchantmentType
{
    public static readonly IReadOnlyList<string> Known = ["Aura"];
}

public static class BasicLandType
{
    public const string Plains = "Plains";
    public const string Island = "Island";
    public const string Swamp = "Swamp";
    public const string Mountain = "Mountain";
    public const string Forest = "Forest";

    public static readonly IReadOnlyList<string> All = [Plains, Island, Swamp, Mountain, Forest];

    /// <summary>The color a basic land subtype produces, or null if the subtype is not a basic land type.</summary>
    public static ManaColor? ProducedColor(string subtype) => subtype switch
    {
        Plains => ManaColor.White,
        Island => ManaColor.Blue,
        Swamp => ManaColor.Black,
        Mountain => ManaColor.Red,
        Forest => ManaColor.Green,
        _ => null,
    };

    /// <summary>
    /// Every color this type line's land subtypes can produce, in color order. A land with several
    /// basic types has one intrinsic mana ability per type.
    /// </summary>
    public static IReadOnlyList<ManaColor> ProducedColors(TypeLine typeLine)
    {
        ArgumentNullException.ThrowIfNull(typeLine);

        List<ManaColor> colors = [];
        foreach (string subtype in typeLine.Subtypes)
        {
            if (ProducedColor(subtype) is { } color && !colors.Contains(color))
            {
                colors.Add(color);
            }
        }

        colors.Sort();
        return colors;
    }
}

/// <summary>
/// The sets subtypes come in (CR 205.3): creature types, artifact types, land types and so on, each correlated with the
/// card types that may have it (CR 205.3d). What an effect that <em>sets</em> a subtype needs: the new subtype
/// replaces the existing subtypes from its own set and leaves every other set alone (CR 205.1a), so "becomes a
/// Wurm" sheds an artifact creature's Thopter and keeps its Artifact, and a land creature's Forest.
/// <para>
/// Planeswalker, planar and dungeon types are not listed yet: <see cref="Of"/> answers null for them, and an effect
/// that sets one is refused rather than guessed at.
/// </para>
/// </summary>
public sealed class SubtypeSet
{
    private SubtypeSet(string name, IReadOnlyList<CardType> cardTypes, IReadOnlyList<string> members)
    {
        Name = name;
        CardTypes = cardTypes;
        Members = members;
    }

    /// <summary>What the rules call this set's members ("creature types").</summary>
    public string Name { get; }

    /// <summary>The card types that may have a subtype of this set (CR 205.3d).</summary>
    public IReadOnlyList<CardType> CardTypes { get; }

    /// <summary>Every subtype in the set.</summary>
    public IReadOnlyList<string> Members { get; }

    /// <summary>Creature types (CR 205.3m), shared by creatures and kindreds.</summary>
    public static readonly SubtypeSet Creature = new("creature types", [CardType.Creature, CardType.Kindred], CreatureType.All);

    /// <summary>Artifact types (CR 205.3g).</summary>
    public static readonly SubtypeSet Artifact = new("artifact types", [CardType.Artifact],
    [
        "Attraction", "Blood", "Bobblehead", "Book", "Clue", "Contraption", "Equipment", "Food", "Fortification", "Gold",
        "Heartwood", "Incubator", "Infinity", "Junk", "Lander", "Map", "Mutagen", "Powerstone", "Spacecraft", "Stone", "Treasure",
        "Vehicle", "Vibranium",
    ]);

    /// <summary>Enchantment types (CR 205.3h).</summary>
    public static readonly SubtypeSet Enchantment = new("enchantment types", [CardType.Enchantment],
    [
        "Aura", "Background", "Cartouche", "Case", "Class", "Curse", "Plan", "Role", "Room", "Rune", "Saga", "Shard",
        "Shrine",
    ]);

    /// <summary>Land types (CR 205.3i).</summary>
    public static readonly SubtypeSet Land = new("land types", [CardType.Land],
    [
        "Cave", "Desert", "Forest", "Gate", "Island", "Lair", "Locus", "Mine", "Mountain", "Plains", "Planet",
        "Power-Plant", "Sphere", "Swamp", "Tower", "Town", "Urza’s",
    ]);

    /// <summary>Spell types (CR 205.3k), shared by instants and sorceries.</summary>
    public static readonly SubtypeSet Spell = new("spell types", [CardType.Instant, CardType.Sorcery],
        ["Adventure", "Arcane", "Lesson", "Omen", "Trap"]);

    /// <summary>Battle types (CR 205.3q).</summary>
    public static readonly SubtypeSet Battle = new("battle types", [CardType.Battle], ["Siege"]);

    private static readonly SubtypeSet[] Listed = [Creature, Artifact, Enchantment, Land, Spell, Battle];

    /// <summary>The set a subtype belongs to, or null for one in no listed set.</summary>
    public static SubtypeSet? Of(string subtype)
    {
        foreach (SubtypeSet set in Listed)
        {
            foreach (string member in set.Members)
            {
                if (string.Equals(member, subtype, StringComparison.Ordinal))
                {
                    return set;
                }
            }
        }

        return null;
    }

    public override string ToString() => Name;
}
