using Cantrip.Core;
using Cantrip.Protocol;

namespace Cantrip.Client.Presentation;

/// <summary>
/// Canned positions, one per dock the client can show, and the transport that feeds them to a real
/// <see cref="GameClient"/>.
/// <para>
/// Here so a picture of a prompt does not need a game. Reaching a particular prompt by playing was a seed hunt —
/// deal, play, hope, repeat — and it got worse every time the demo decks changed, because the seed that used to
/// reach the prompt stopped doing so. A fixture reaches every one of them, in order, in a single run.
/// </para>
/// <para>
/// These are server messages and nothing else: the client is built the ordinary way over a transport that happens
/// to have the messages already, so the welcome, the snapshot and the prompt travel the path they always do and
/// the dock is drawn by the same <c>Fill*</c> code a real game draws it with. That is what makes the pictures worth
/// looking at — a wording or layout fault in that code shows up here rather than being papered over by a fixture
/// that drew itself.
/// </para>
/// </summary>
public static class PromptFixtures
{
    /// <summary>The seat the pictures are taken from.</summary>
    public const int Viewer = 1;

    /// <summary>The other seat. Two players: a third rail would crowd out the thing being pictured.</summary>
    public const int Rival = 2;

    // Handles. Fixed rather than allocated, so a fixture can name the card it wants to do something to.
    private const int MySwamp = 1;
    private const int MySwampTwo = 2;
    private const int MySwampTapped = 3;
    private const int MyLurker = 4;
    private const int MyBrute = 5;
    private const int TheirPlains = 6;
    private const int TheirPlainsTwo = 7;
    private const int TheirRecruit = 8;
    private const int TheirSentinel = 9;
    private const int HandRitual = 10;
    private const int HandBrute = 11;
    private const int HandTwist = 12;
    private const int HandSwamp = 13;
    private const int MyGrave = 14;
    private const int TheirGrave = 15;
    /// <summary>A handle no object in the fixtures' view has: a source that has left, which the view never held.</summary>
    private const int UnseenSource = 999;
    /// <summary>Every creature type (CR 205.3m), the longest list of named options a card offers.</summary>
    public static readonly IReadOnlyList<string> CreatureTypes =
    [
        "Advisor", "Aenar", "Aetherborn", "Alien", "Ally", "Andorian", "Android", "Angel", "Antelope", "Ape", "Archer", "Archon", "Armadillo",
        "Army", "Artificer", "Assassin", "Assembly-Worker", "Astartes", "Atog", "Aurochs", "Avatar", "Azra",
        "Badger", "Bajoran", "Balloon", "Barbarian", "Bard", "Basilisk", "Bat", "Bear", "Beast", "Beaver", "Beeble",
        "Beholder", "Berserker", "Betazoid", "Bird", "Bison", "Blinkmoth", "Boar", "Bolian", "Borg", "Bringer", "Brushwagg", "Caitian", "Camarid", "Camel",
        "Capybara", "Cardassian", "Caribou", "Carrier", "Cat", "Centaur", "Child", "Chimera", "Citizen", "Cleric", "Clown",
        "Cockatrice", "Construct", "Coward", "Coyote", "Crab", "Crocodile", "Custodes", "Cyberman", "Cyclops",
        "C’tan", "Dalek", "Dauthi", "Demigod", "Demon", "Denobulan", "Deserter", "Detective", "Devil", "Dinosaur", "Djinn",
        "Doctor", "Dog", "Dragon", "Drake", "Dreadnought", "Drix", "Drone", "Druid", "Dryad", "Dwarf", "Echidna",
        "Efreet", "Egg", "El-Aurian", "Elder", "Eldrazi", "Elemental", "Elephant", "Elf", "Elk", "Employee", "Eternal", "Excalbian", "Eye",
        "Faerie", "Ferengi", "Ferret", "Fish", "Flagbearer", "Fox", "Fractal", "Frog", "Fungus", "Gamer", "Gamma", "Gargoyle",
        "Germ", "Giant", "Giraffe", "Gith", "Glimmer", "Gnoll", "Gnome", "Goat", "Goblin", "God", "Golem", "Gorgon", "Gorn",
        "Graveborn", "Gremlin", "Griffin", "Guest", "Hag", "Halfling", "Hamster", "Harpy", "Hedgehog", "Hellion",
        "Hero", "Hippo", "Hippogriff", "Hirogen", "Homarid", "Homunculus", "Horror", "Horse", "Human", "Hydra", "Hyena",
        "Illusion", "Illyrian", "Imp", "Incarnation", "Inhuman", "Inkling", "Inquisitor", "Insect", "Jackal", "Jellyfish",
        "Juggernaut", "Kangaroo", "Kavu", "Kelpien", "Kirin", "Kithkin", "Klingon", "Knight", "Kobold", "Kor", "Kraken", "Kree", "Kwejian", "Lamia",
        "Lammasu", "Lanthanite", "Leech", "Lemur", "Leviathan", "Lhurgoyf", "Licid", "Lizard", "Llama", "Lobster", "Manticore",
        "Masticore", "Medusan", "Mercenary", "Merfolk", "Metathran", "Minion", "Minotaur", "Mite", "Mole", "Monger",
        "Mongoose", "Monk", "Monkey", "Moogle", "Moonfolk", "Mount", "Mouse", "Mutant", "Myr", "Mystic", "Na'kuhl", "Nautilus",
        "Necron", "Nephilim", "Nightmare", "Nightstalker", "Ninja", "Noble", "Noggle", "Nomad", "Nymph", "Ocampa", "Octopus", "Officer",
        "Ogre", "Ooze", "Orb", "Orc", "Orgg", "Orion", "Otter", "Ouphe", "Ox", "Oyster", "Pangolin", "Peasant", "Pegasus",
        "Pentavite", "Performer", "Pest", "Phelddagrif", "Phoenix", "Phyrexian", "Pilot", "Pincher", "Pirate",
        "Plant", "Platypus", "Porcupine", "Possum", "Praetor", "Primarch", "Prism", "Processor", "Q", "Qu", "Rabbit",
        "Raccoon", "Ranger", "Rat", "Rebel", "Reflection", "Rhino", "Rigger", "Robot", "Rogue", "Romulan", "Sable",
        "Salamander", "Samurai", "Sand", "Saproling", "Satyr", "Scarecrow", "Scientist", "Scion", "Scorpion",
        "Scout", "Sculpture", "Seal", "Serf", "Serpent", "Servo", "Shade", "Shaman", "Shapeshifter", "Shark",
        "Sheep", "Shi’ar", "Siren", "Skeleton", "Skrull", "Skunk", "Slith", "Sliver", "Sloth", "Slug", "Snail",
        "Snake", "Soldier", "Soltari", "Sorcerer", "Spawn", "Specter", "Spellshaper", "Sphinx", "Spider", "Spike",
        "Spirit", "Splinter", "Sponge", "Spy", "Squid", "Squirrel", "Starfish", "Surrakar", "Survivor", "Symbiote",
        "Synth", "Talosian", "Tamarian", "Tellarite", "Tentacle", "Tetravite", "Thalakos", "Tholian", "Thopter", "Thrull", "Tiefling", "Time Lord", "Tosk", "Toy",
        "Treefolk", "Tribble", "Trill", "Trilobite", "Triskelavite", "Troll", "Turtle", "Tyranid", "Unicorn", "Utrom", "Vampire",
        "Varmint", "Vau-N'Akat", "Vedalken", "Villain", "Volver", "Vorta", "Vulcan", "Wall", "Walrus", "Warlock", "Warrior", "Weasel", "Weird",
        "Werewolf", "Whale", "Wizard", "Wolf", "Wolverine", "Wombat", "Worm", "Wraith", "Wurm", "Xindi", "Yeti", "Zombie",
        "Zubera"
    ];

    private const int OnStack = 16;
    private const int OnStackTwo = 21;
    private const int TheirHandCard = 17;
    private const int TheirHandCardTwo = 18;
    private const int MyColonnade = 19;
    private const int TheirSaheeli = 20;
    private const int MyTundra = 22;
    private const int MyIsland = 23;
    private const int HandHerald = 24;
    private const int MyFaceDown = 25;
    private const int TheirFaceDown = 26;
    private const int MyKey = 27;
    private const int MyKeyTwo = 28;
    private const int MyVault = 29;
    private const int FirstLibraryCard = 30;
    private const int FirstLookedAt = 50;
    private const int FirstBuried = 60;
    private const int FirstVoided = 70;
    private const int FirstHidden = 80;
    private const int MyResources = 90;
    private const int MyStripMine = 91;
    private const int MyStar = 92;
    private const int TheirCurse = 93;
    private const int FirstOpening = 94;
    private const int MySiege = 110;

    /// <summary>
    /// Every fixture, in the order the gallery renders them: the board's own prompts first, then the ones that
    /// open the choice window, then combat, then the two states that are not prompts at all.
    /// </summary>
    /// <summary>
    /// Gurmag Angler cast with a kicker of {2}, announcing <paramref name="kicks"/> kicks so far (CR 601.2b): nothing
    /// is paid yet, and <paramref name="affordable"/> is as many as the mana on hand reaches.
    /// </summary>
    private static PayCostsPromptMessage Kicking(string total, int kicks, bool multi, int affordable)
    {
        List<ActionOptionMessage> options = [];

        if (multi || kicks == 0)
        {
            options.Add(new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = new ViewHandle(HandBrute) });
        }

        if (kicks > 0)
        {
            options.Add(new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = new ViewHandle(HandBrute) });
        }

        options.Add(new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandBrute) });
        options.Add(new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) });

        return new PayCostsPromptMessage
        {
            Player = Viewer,
            Spell = new ViewHandle(HandBrute),
            Total = total,
            Remaining = total,
            HasX = true,
            X = kicks,
            Kicker = "{2}",
            Multikicker = multi,
            AnnouncesXOrKicks = true,
            MostAffordable = affordable,
            Options = options,
        };
    }

    /// <summary>
    /// Kolaghan's Command given a buyback of {2} (CR 702.27), announced <paramref name="bought"/> times: once or not at
    /// all, so the stepper has only the one move left to offer.
    /// </summary>
    private static PayCostsPromptMessage BuyingBack(string total, int bought) =>
        new()
        {
            Player = Viewer,
            Spell = new ViewHandle(HandRitual),
            Total = total,
            Remaining = total,
            HasX = true,
            X = bought,
            Kicker = "{2}",
            Buyback = true,
            AnnouncesXOrKicks = true,
            MostAffordable = 1,
            Options =
            [
                new ActionOptionMessage
                {
                    Kind = bought == 0 ? ActionKindView.IncreaseX : ActionKindView.DecreaseX,
                    Source = new ViewHandle(HandRitual),
                },
                new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandRitual) },
                new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandRitual) },
            ],
        };

    private static readonly List<string> Order =
    [
        "lobby",
        "priority",
        "priority-stack",
        "priority-stack-many",
        "priority-stack-modal-trigger",
        "priority-animated-land",
        "priority-sacrifice-for-mana",
        "priority-tap-for-mana",
        "targeting",
        "targeting-optional",
        "discard-one",
        "discard-many",
        "paying",
        "paying-dual",
        "paying-waterbend",
        "paying-improvise",
        "paying-convoke",
        "paying-delve",
        "paying-no-mana",
        "choosing-mana",
        "choosing-mana-for-ability",
        "choosing-x",
        "announcing-x",
        "announcing-life-x",
        "multikicking",
        "multikicking-owed",
        "kicking",
        "variable-kicker-unpaid",
        "variable-kicker-zero",
        "kicking-colorless-owed",
        "buying-back",
        "bought-back",
        "choose-target",
        "choose-target-optional",
        "choose-target-several",
        "choose-target-up-to-several",
        "choose-target-mode",
        "choose-target-range",
        "divide-targets",
        "divide-targets-capped",
        "divide-targets-optional",
        "choose-not-targeted",
        "choose-any-number",
        "choose-any-number-one",
        "unless-pays",
        "unless-pays-short",
        "unless-pays-copy",
        "unless-pays-untap",
        "unless-pays-return",
        "unless-pays-return-gone",
        "may",
        "unless-pays-life",
        "may-promise-gift",
        "may-promise-gift-tapped-fish",
        "unless-pays-life-to-look-again",
        "unless-pays-life-to-return",
        "may-reveal-again",
        "may-reveal-for-ripple",
        "unless-pays-life-unless-countered",
        "unless-pays-mana-and-life",
        "may-discard-unless-countered",
        "may-cast",
        "may-cast-transformed",
        "may-cast-exiled",
        "may-cast-paradigm-copy",
        "may-cast-discarded",
        "may-play-land-during-resolution",
        "may-cast-during-resolution",
        "may-cast-during-resolution-paid",
        "may-cast-found-or-to-hand",
        "choose-play-way",
        "may-put-on-top",
        "may-counter-or-cast",
        "may-copy",
        "may-attach",
        "may-return-and-attach",
        "may-create-treasure",
        "may-draw",
        "may-shuffle-into-library",
        "may-shuffle-no-cards",
        "may-deal-damage",
        "may-explore-graveyard",
        "may-put-on-bottom",
        "may-pay-energy",
        "may-pay-energy-cost",
        "may-protect-from-colorless",
        "may-choose-odd-or-even",
        "may-reveal-until-creature",
        "may-reveal-until-shares-creature-type",
        "may-begin-on-battlefield",
        "may-reveal-for-miracle",
        "may-cast-for-miracle-cost",
        "may-cast-for-madness-cost",
        "may-cast-for-rebound",
        "may-remove-a-counter-to-destroy",
        "may-have-controller-draw",
        "may-have-controller-draw-one",
        "may-take-damage-instead-of-return",
        "modes-one",
        "separate-piles",
        "choose-pile",
        "additional-cost",
        "phyrexian-life",
        "venture-room",
        "choose-option",
        "choose-option-word",
        "remove-counters-up-to",
        "choose-option-long",
        "modes-many",
        "modes-range",
        "choose-replacement",
        "choose-replacement-unseen-source",
        "name-card",
        "name-card-empty-hand",
        "choose-starting-player",
        "choose-starting-player-multiplayer",
        "mulligan",
        "mulligan-again",
        "choose-put-on-bottom",
        "choose-put-on-bottom-effect",
        "waiting-to-start",
        "choose-player",
        "choose-player-protection",
        "choose-player-gift",
        "choose-player-protector",
        "choose-color",
        "choose-color-protection",
        "choose-colors-several",
        "order-triggers",
        "search",
        "search-single-card",
        "search-several",
        "search-exile",
        "search-exile-face-down",
        "search-cast",
        "search-land-or-hand",
        "choose-discard",
        "choose-discard-up-to",
        "choose-from-their-hand",
        "choose-from-their-hand-up-to",
        "choose-put-on-top",
        "choose-splice",
        "choose-legend",
        "choose-sacrifice",
        "choose-sacrifice-up-to",
        "choose-sacrifice-several",
        "choose-exile-cost",
        "choose-exile-graveyard",
        "choose-imprint",
        "choose-plot",
        "choose-exile-hand",
        "choose-return-to-hand",
        "choose-untap",
        "choose-attach",
        "choose-attach-one",
        "choose-tap-to-pay",
        "choose-collect-evidence",
        "choose-put-onto-battlefield",
        "choose-put-onto-battlefield-revealed",
        "choose-put-onto-battlefield-within-total",
        "choose-put-into-hand",
        "choose-put-into-their-hand",
        "choose-cast",
        "choose-keep",
        "choose-destroy",
        "choose-vote",
        "choose-put-into-graveyard",
        "choose-play",
        "choose-nothing",
        "choose-empty",
        "choose-mixed-zones",
        "choose-return-effect",
        "choose-sacrifice-cost",
        "choose-order-replacements",
        "scry",
        "distribute-empty",
        "distribute-one",
        "surveil",
        "arrange-in-any-order",
        "arrange-onto-the-bottom",
        "distribute-to-hand",
        "distribute-tight",
        "distribute-each-card-type",
        "distribute-permanent-cards-only",
        "manifest-dread",
        "attackers",
        "attackers-planeswalker",
        "attackers-battle",
        "attackers-goaded",
        "blockers",
        "blockers-restricted",
        "blockers-one-at-most",
        "blockers-two-each",
        "assign-damage",
        "player-aura",
        "waiting",
        "priority-controlling",
        "controlled-waiting",
        "game-over",
    ];

    public static IReadOnlyList<string> Names => Order;

    /// <summary>The fixtures asked while a spell is still being announced, so they offer Cancel.</summary>
    private static readonly HashSet<string> Announcing = ["modes-one", "additional-cost", "phyrexian-life","modes-many", "modes-range", "choose-target-mode", "choose-target-range", "choose-exile-cost", "choose-exile-graveyard", "choose-splice", "divide-targets", "divide-targets-capped"];

    public static bool Known(string name) => Order.Contains(name, StringComparer.Ordinal);

    /// <summary>
    /// The messages a server would have sent to put this client in front of the named dock: a welcome, the
    /// position, and the prompt — or, for the two fixtures that are not prompts, whatever stands in its place.
    /// </summary>
    /// <exception cref="ArgumentOutOfRangeException">The name is not one of <see cref="Names"/>.</exception>
    public static IReadOnlyList<ServerMessage> Build(string name)
    {
        PlayerView view = Board();
        int request = Math.Max(1, Order.IndexOf(name) + 1);

        if (name == "game-over")
        {
            view.Outcome = GameOutcomeView.TeamWon;
            view.WinningTeam = Viewer;
            view.PriorityPlayer = 0;
            view.Players[1].Life = 0;
            view.Players[1].Status = PlayerStatusView.Lost;

            return
            [
                new WelcomeMessage { Seat = Viewer, PlayerNames = ["You", "Bot 2"] },
                new StateMessage { View = view },

                // What took the loser out, which the game-over message itself does not carry: the panel announcing
                // the result reads it off the server's log, as the log panel does.
                new EventsMessage
                {
                    Sequence = 1,
                    Events = [new PlayerLostEvent { Player = Rival, Reason = LossReasonView.Life }],
                    Sequences = [1],
                },
                new LogMessage
                {
                    Entries =
                    [
                        new LogEntryView
                        {
                            Number = 1,
                            Sequence = 1,
                            Turn = view.Turn,
                            ActivePlayer = view.ActivePlayer,
                            Step = view.Step,
                            Text = "Bot 2 loses the game: their life total is 0.",
                            Kind = LogEntryKind.PlayerLost,
                        },
                    ],
                },
                new GameOverMessage { Outcome = GameOutcomeView.TeamWon, WinningTeam = Viewer },
            ];
        }

        PromptPayload? payload = Fill(name, view);

        List<ServerMessage> messages =
        [
            new WelcomeMessage { Seat = Viewer, PlayerNames = ["You", "Bot 2"] },
            new StateMessage { View = view },
        ];

        if (name == "lobby")
        {
            messages.Add(new TableMessage
            {
                Phase = TablePhase.Setup, Capacity = 2, HostSeat = Viewer,
                Players =
                [
                    new TablePlayerView { Seat = Viewer, Name = "You", Deck = "starter-blue", Connected = true },
                    new TablePlayerView { Seat = Rival, Name = "Bot 2", Deck = "starter-black", Connected = true },
                ],
                Decks = ["starter-blue", "starter-black"],
                AvailableSets = ["CUBE", "DEMO"], SelectedSets = ["CUBE"],
                Detail = "Choose your deck and ready up. The host chooses sets before anyone is ready.",
            });
        }

        if (payload is not null)
        {
            messages.Add(new PromptMessage
            {
                RequestId = request, Prompt = payload, MayCancel = Announcing.Contains(name),
                Presentation = Presentation(payload, view, name),
                // Asked of the player this seat controls (CR 723.5): their decision, sent to this seat.
                ForSeat = name == "priority-controlling" ? Rival : 0,
            });
        }

        return messages;
    }

    /// <summary>
    /// The prompt for a fixture, after any change it needs to the position. Null for a fixture that is a state
    /// rather than a question.
    /// </summary>

    private static PromptPresentationContext? Presentation(PromptPayload prompt, PlayerView view, string fixture)
    {
        PromptPresentationContext? context = prompt switch
        {
            ChooseCardsPromptMessage choose => new()
            {
                Action = CardChoicePresentation.ActionFor(choose.Purpose),
                Cause = choose.Purpose is CardChoicePurposeView.KeepLegend or CardChoicePurposeView.OrderSimultaneous
                    ? PromptCauseView.Rule
                    : fixture is "choose-sacrifice-cost" or "choose-return-to-hand"
                        || choose.Purpose is CardChoicePurposeView.ExileToPay or CardChoicePurposeView.TapToPay or CardChoicePurposeView.TapToReduce
                        ? PromptCauseView.Cost : PromptCauseView.Effect,
                Chooser = choose.Player,
                Owner = choose.Owner,
                Source = choose.Source,
                Minimum = choose.UpTo ? 0 : choose.Count,
                Maximum = choose.Count,
                Ordered = choose.Purpose is CardChoicePurposeView.PutOnTop or CardChoicePurposeView.Splice,
                Origins = Origins(choose.Cards),
            },
            DistributeCardsPromptMessage distribute => new()
            {
                Action = PromptActionView.Distribute, Chooser = distribute.Player, Owner = distribute.Player,
                Source = distribute.Source, Origins = Origins(distribute.Cards),
                Minimum = distribute.Cards.Count, Maximum = distribute.Cards.Count,
                Ordered = distribute.Destinations.Any(destination => destination.Ordered),
            },
            PayCostsPromptMessage pay => new()
            {
                Action = PromptActionView.Pay, Cause = PromptCauseView.Cost,
                Chooser = pay.Player, Owner = pay.Player, Source = pay.Spell,
            },
            _ => null,
        };
        return context;

        List<PromptOriginView> Origins(IEnumerable<ViewHandle> handles) =>
            [.. handles.Select(handle => view.Objects.First(card => card.Handle == handle))
                .Select(card => (card.Zone, card.ZoneOwner)).Distinct()
                .Select(origin => new PromptOriginView { Zone = origin.Zone, Owner = origin.ZoneOwner })];
    }


    private static PromptPayload? Fill(string name, PlayerView view)
    {
        switch (name)
        {
            case "lobby":
                return null;
            case "waiting":
                // Nothing is being asked of this seat: the other player is answering something of their own.
                view.PriorityPlayer = Rival;
                return null;

            case "priority-controlling":
                // This seat controls Bot 2 during Bot 2's turn (CR 723: Mindslaver): Bot 2 is the active player and holds
                // priority, its hand is this seat's to see (CR 723.4), and its priority is asked of this seat (CR 723.5), with
                // Bot 2's own land and Plains to play.
                view.ActivePlayer = Rival;
                view.PriorityPlayer = Rival;
                view.Players[1].ControlledBy = Viewer;
                view.Players[1].PendingEffects = ["You control them during their turn."];
                TheirHand(view);
                return new PriorityPromptMessage
                {
                    Player = Rival,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = new ViewHandle(TheirHandCardTwo) },
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.ActivateAbility, Source = new ViewHandle(TheirPlains), Produces = [ManaTypeView.White],
                            Description = "{T}: Add {W}.", CostText = "{T}",
                        },
                    ],
                };

            case "controlled-waiting":
                // This seat's own turn, controlled by Bot 2 (CR 723): nothing is asked of this seat, and Bot 2 answers for it.
                view.Players[0].ControlledBy = Rival;
                view.Players[0].PendingEffects = ["Bot 2 controls you during your turn."];
                return null;

            case "player-aura":
                // A board-only fixture: a Curse stands in its controller's row and names the player it enchants.
                view.Objects.Add(new ObjectView
                {
                    Handle = new ViewHandle(TheirCurse), Zone = ZoneKindView.Battlefield,
                    Kind = ObjectKindView.Card, Owner = Rival, Controller = Rival,
                    Name = "Curse of the Bloody Tome", CardId = "Curse of the Bloody Tome",
                    ManaCost = "{2}{U}", ManaValue = 3, CardTypes = [CardTypeView.Enchantment],
                    Subtypes = ["Aura", "Curse"], Colors = [ColorView.Blue],
                    OracleText = "Enchant player\nAt the beginning of enchanted player's upkeep, that player mills two cards.",
                    AttachedToPlayer = Viewer,
                });
                Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(TheirCurse));
                return null;

            case "priority":
                return Priority();

            case "priority-stack":
                // Their spell is waiting to resolve, which is what the window to respond is for.
                Announce(view, "Swords to Plowshares", "{W}", "Exile target creature. Its controller gains life equal to its power.", Rival);
                return Priority();

            case "priority-stack-many":
                // Two things waiting, which is when Resolve All is offered beside Resolve.
                Announce(view, "Mind Twist", "{X}{B}", "Target player discards X cards at random.", Viewer, CardTypeView.Sorcery);
                Announce(view, "Swords to Plowshares", "{W}", "Exile target creature. Its controller gains life equal to its power.", Rival, handle: OnStackTwo);
                return Priority();

            case "priority-stack-modal-trigger":
                // Their Bejeweled Warg's modal trigger, its mode chosen as it went on the stack (CR 603.3c): the mode
                // has no target, so only the ability's own chosen modes say which one is waiting (CR 700.2).
                Trigger(view, "Bejeweled Warg", "Whenever Bejeweled Warg deals combat damage to a player, choose one \u2014 put a +1/+1 counter on target Wolf you control; or create a Treasure token.");
                Object(view, OnStack).Owner = Rival;
                Object(view, OnStack).Controller = Rival;
                Object(view, OnStack).Colors = [ColorView.Green];
                Object(view, OnStack).ChosenModes = ["Create a Treasure token."];
                return Priority();

            case "priority-animated-land":
                // A land that layer 4 has made a creature and layer 7b has given a size: it draws as a creature card
                // while still being a land, and its animation ability is the longest row the action menu ever shows.
                return AnimatedLand(view);

            case "priority-sacrifice-for-mana":
                // Squandered Resources, whose one mana is of a type the sacrificed land could produce (CR 106.7):
                // one option per candidate, and the Strip Mine's is colorless alone. Clicking the card picks the land
                // on the board, and the Strip Mine's then adds {C} where the Swamp's adds {B}.
                return SacrificeForMana(view);

            case "priority-tap-for-mana":
                // Urza, Lord High Artificer, clicked: which artifact to tap is picked on the board, the way a target is.
                return TapForMana(view);

            case "targeting":
                return Priority();

            case "targeting-optional":
            {
                // "Up to one target" (CR 601.2c), as Cryogen Relic's stun counter: the same dock, with declining
                // offered beside cancelling, because aiming at nothing is a choice the ability makes rather than a
                // way out of taking it.
                PriorityPromptMessage declinable = Priority();
                declinable.Options.First(CardActions.NeedsATarget).TargetsOptional = true;
                return declinable;
            }

            case "discard-one":
                return new DiscardPromptMessage
                {
                    Player = Viewer,
                    Count = 1,
                    MaximumHandSize = 7,
                    Hand = Hand(),
                };

            case "discard-many":
                return new DiscardPromptMessage
                {
                    Player = Viewer,
                    Count = 3,
                    MaximumHandSize = 7,
                    Hand = Hand(),
                };

            case "paying":
                Float(view, ManaTypeView.Black, 1);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{2}{B}",
                    Remaining = "{2}",
                    RemainingGeneric = 2,
                    Options =
                    [
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        new ActionOptionMessage { Kind = ActionKindView.UndoManaAbility, Source = new ViewHandle(MySwampTapped) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "paying-dual":
                // A dual land while {U}{W} is owed: the one position where which mana ability to use is a decision,
                // and so the one that shows what the Auto-mana line in the dock says.
                DualLands(view);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandHerald),
                    Total = "{2}{U}{W}",
                    Remaining = "{1}{U}{W}",
                    RemainingGeneric = 1,
                    RemainingColored = [ColorView.Blue, ColorView.White],
                    Options =
                    [
                        TapFor(MyTundra, ManaTypeView.White),
                        TapFor(MyTundra, ManaTypeView.Blue),
                        TapFor(MyIsland, ManaTypeView.Blue),
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandHerald) },
                    ],
                };

            case "paying-waterbend":
                // A waterbend cost (CR 701.67a): the creatures that could pay {1} each by tapping are offered beside the
                // lands, so the dock says so.
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(MyBrute),
                    Total = "{8}",
                    Remaining = "{8}",
                    RemainingGeneric = 8,
                    Options =
                    [
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyLurker) },
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyBrute) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(MyBrute) },
                    ],
                };

            case "paying-improvise":
                // Improvise (CR 702.126a): the same tap that pays {1}, but artifacts only — so the dock's line reads
                // "or tap an artifact", where waterbend's above reads "or tap an artifact or creature".
                Artifacts(view);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{5}{U}",
                    Remaining = "{5}{U}",
                    RemainingGeneric = 5,
                    RemainingColored = [ColorView.Blue],
                    Options =
                    [
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyKey) },
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyKeyTwo) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "paying-convoke":
                // Convoke (CR 702.51a): each creature says what it could pay — {1} or a mana of its own color — so the
                // dock reads "or tap a creature to help pay" rather than improvise's "for {1}".
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{3}{W}",
                    Remaining = "{3}{W}",
                    RemainingGeneric = 3,
                    RemainingColored = [ColorView.White],
                    Options =
                    [
                        Tap(MySwamp),
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyLurker), Description = "Tap Vampire Nighthawk to pay {1} or {B}" },
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyBrute), Description = "Tap Gurmag Angler to pay {1} or {B}" },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "paying-delve":
                // Delve (CR 702.66a): the graveyard cards that could pay {1} each by being exiled are offered beside the
                // lands, so the dock says so.
                List<ViewHandle> buried = Graveyard(view, 4);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{6}{U}{U}",
                    Remaining = "{6}{U}{U}",
                    RemainingGeneric = 6,
                    RemainingColored = [ColorView.Blue, ColorView.Blue],
                    Options =
                    [
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        .. buried.Select(card => new ActionOptionMessage { Kind = ActionKindView.ExileToPay, Source = card }),
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "paying-no-mana":
                // "You can't spend mana to cast this spell" (CR 601.2h, 106.4), with convoke and delve, as Hogaak's: the
                // Swamp's mana ability is still offered (CR 601.2g) and a black floats, but the dock names only the taps
                // and the exiles, since no mana pays it.
                Float(view, ManaTypeView.Black, 1);
                List<ViewHandle> delved = Graveyard(view, 3);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{5}{B/G}{B/G}",
                    Remaining = "{5}{B/G}{B/G}",
                    RemainingGeneric = 5,
                    RemainingHybrid =
                    [
                        new HybridSymbolView { Colors = [ColorView.Black, ColorView.Green] },
                        new HybridSymbolView { Colors = [ColorView.Black, ColorView.Green] },
                    ],
                    SpendsNoMana = true,
                    Options =
                    [
                        Tap(MySwamp),
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyLurker), Description = "Tap Vampire Nighthawk to pay {1} or {B}" },
                        new ActionOptionMessage { Kind = ActionKindView.TapToPay, Source = new ViewHandle(MyBrute), Description = "Tap Gurmag Angler to pay {1} or {B}" },
                        .. delved.Select(card => new ActionOptionMessage { Kind = ActionKindView.ExileToPay, Source = card }),
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "choosing-mana":
                // Two black and two red for {2}{B}: a black and a red are spoken for whichever way it is paid, and
                // the last mana is either. A click on the black chip or the red chip casts.
                Float(view, ManaTypeView.Black, 2);
                Float(view, ManaTypeView.Red, 2);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{2}{B}",
                    Remaining = "{1}",
                    RemainingGeneric = 1,
                    Committed =
                    [
                        new RestrictedManaView { Type = ManaTypeView.Black, Amount = 1 },
                        new RestrictedManaView { Type = ManaTypeView.Red, Amount = 1 },
                    ],
                    Options =
                    [
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.AssignMana,
                            Source = new ViewHandle(HandBrute),
                            Produces = [ManaTypeView.Red],
                        },
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.AssignMana,
                            Source = new ViewHandle(HandBrute),
                            Produces = [ManaTypeView.Black],
                        },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "choosing-mana-for-ability":
                // Chromatic Star's own {1} (CR 602.2b, 601.2h) with a blue and a black floating: either pays it, and
                // which is kept is the player's to say, on the same dock a spell's cost uses.
                Float(view, ManaTypeView.Blue, 1);
                Float(view, ManaTypeView.Black, 1);
                ChromaticStar(view);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(MyStar),
                    Total = "{1}",
                    Remaining = "{1}",
                    RemainingGeneric = 1,
                    Options =
                    [
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.AssignMana,
                            Source = new ViewHandle(MyStar),
                            Produces = [ManaTypeView.Blue],
                        },
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.AssignMana,
                            Source = new ViewHandle(MyStar),
                            Produces = [ManaTypeView.Black],
                        },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(MyStar) },
                    ],
                };

            case "choosing-x":
                Float(view, ManaTypeView.Black, 3);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandTwist),
                    Total = "{X}{B}",
                    Remaining = "{0}",
                    HasX = true,
                    X = 2,
                    Committed = [new RestrictedManaView { Type = ManaTypeView.Black, Amount = 3 }],
                    Options =
                    [
                        Tap(MySwamp),
                        new ActionOptionMessage
                        {
                            Kind = ActionKindView.AssignMana,
                            Source = new ViewHandle(MySwamp),
                            Produces = [ManaTypeView.Black],
                        },
                        new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandTwist) },
                    ],
                };

            case "announcing-x":
                // The {X} of the spell "choosing-x" pays for, announced first (CR 601.2b): before its targets and before
                // anything is paid, so the dock says what the total comes to and how far the mana on hand reaches.
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandTwist),
                    Total = "{2}{B}",
                    Remaining = "{2}{B}",
                    HasX = true,
                    X = 2,
                    AnnouncesXOrKicks = true,
                    MostAffordable = 3,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandTwist) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandTwist) },
                    ],
                };

            case "announcing-life-x":
                // An X paid in life ("pay X life", CR 119.4), announced before the targets: the mana owed does not move,
                // and the most the dock advises is what leaves the player alive (CR 704.5a).
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandRitual),
                    Total = "{2}{B}",
                    Remaining = "{2}{B}",
                    HasX = true,
                    X = 3,
                    AnnouncesXOrKicks = true,
                    XPaidInLife = true,
                    MostAffordable = 19,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = new ViewHandle(HandRitual) },
                        new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = new ViewHandle(HandRitual) },
                        new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandRitual) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandRitual) },
                    ],
                };

            case "multikicking":
                // Multikicker {2} (CR 702.33c), announced as two kicks (CR 601.2b): the count reads as kicks, not as an X.
                Float(view, ManaTypeView.Black, 7);
                return Kicking(total: "{6}{B}", kicks: 2, multi: true, affordable: 3);

            case "multikicking-owed":
                // Announced as one kick more than the mana on hand reaches: legal, and the payment will say so.
                Float(view, ManaTypeView.Black, 4);
                return Kicking(total: "{4}{B}", kicks: 1, multi: true, affordable: 0);

            case "kicking":
                // Plain kicker {2}, not kicked yet: once is the most, so the stepper stops at kicked.
                Float(view, ManaTypeView.Black, 3);
                return Kicking(total: "{2}{B}", kicks: 0, multi: false, affordable: 1);

            case "variable-kicker-unpaid":
            case "variable-kicker-zero":
                // The same chosen X=0 has two legal meanings when the kicker itself contains {X} (CR 702.33).
                bool kickedZero = name == "variable-kicker-zero";
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = kickedZero ? "{4}{B}" : "{2}{B}",
                    Remaining = kickedZero ? "{4}{B}" : "{2}{B}",
                    HasX = true,
                    X = 0,
                    Kicker = "{X}{2}",
                    KickerPaid = kickedZero,
                    VariableKicker = true,
                    AnnouncesXOrKicks = true,
                    MostAffordable = 2,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.IncreaseX, Source = new ViewHandle(HandBrute) },
                        ..(kickedZero ? new[] { new ActionOptionMessage { Kind = ActionKindView.DecreaseX, Source = new ViewHandle(HandBrute) } } : []),
                        new ActionOptionMessage { Kind = ActionKindView.ConfirmCasting, Source = new ViewHandle(HandBrute) },
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "kicking-colorless-owed":
                // Kicker {1}{C} (Sowing Mycospawn), kicked once at the announcement (CR 601.2b) and now being paid, with
                // only green to pay it: everything is covered but the {C}, which only colorless mana pays (CR 107.4c),
                // so the dock says why the green floating is not enough.
                Float(view, ManaTypeView.Green, 5);
                return new PayCostsPromptMessage
                {
                    Player = Viewer,
                    Spell = new ViewHandle(HandBrute),
                    Total = "{4}{G}{C}",
                    Remaining = "{C}",
                    RemainingColorless = 1,
                    Committed = [new RestrictedManaView { Type = ManaTypeView.Green, Amount = 5 }],
                    Options =
                    [
                        Tap(MySwamp),
                        new ActionOptionMessage { Kind = ActionKindView.CancelCasting, Source = new ViewHandle(HandBrute) },
                    ],
                };

            case "buying-back":
                // Buyback {2} (CR 702.27), not bought back yet: the dock offers the buyback, not an X and not a kick.
                Float(view, ManaTypeView.Black, 3);
                return BuyingBack(total: "{1}{B}{R}", bought: 0);

            case "bought-back":
                // Announced as bought back: the card returns to its owner's hand as the spell resolves.
                Float(view, ManaTypeView.Black, 5);
                return BuyingBack(total: "{3}{B}{R}", bought: 1);

            case "choose-target":
                Trigger(view, "Flametongue Kavu", "When this creature enters, it deals 4 damage to target creature.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = new ViewHandle(MyBrute),
                    Text = "It deals 4 damage to target creature.",
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(TheirSentinel) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                    ],
                };

            case "choose-target-optional":
                Trigger(view, "Loran of the Third Path", "When this creature enters, destroy up to one target artifact or enchantment.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = new ViewHandle(MyBrute),
                    Text = "Destroy up to one target artifact or enchantment.",
                    Candidates = [new TargetMessage { Object = new ViewHandle(TheirSentinel) }],
                    Optional = true,
                };

            case "choose-target-several":
                // Candelabra of Tawnos' "Untap X target lands" with X of two: one question, two clicks, and the header
                // counts them off.
                Trigger(view, "Candelabra of Tawnos", "{X}, {T}: Untap X target lands.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),

                    // The board has no Candelabra to point at, so the ability on the stack is what names it.
                    Source = ViewHandle.None,
                    Text = "Untap X target lands.",
                    Count = 2,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(MySwamp) },
                        new TargetMessage { Object = new ViewHandle(MySwampTwo) },
                        new TargetMessage { Object = new ViewHandle(MySwampTapped) },
                        new TargetMessage { Object = new ViewHandle(TheirPlains) },
                    ],
                };

            case "choose-target-up-to-several":
                // Agent Bishop's "each of up to two target creatures": two clicks at most, and none is a legal answer.
                Trigger(view, "Agent Bishop, Man in Black", "At the beginning of combat on your turn, put a +1/+1 counter on each of up to two target creatures.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = ViewHandle.None,
                    Text = "Put a +1/+1 counter on each of up to two target creatures.",
                    Count = 2,
                    Optional = true,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(MyBrute) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                    ],
                };

            case "choose-not-targeted":
                // Comet's "to a creature or player": chosen as the ability resolves rather than targeted (CR 608.2d),
                // so the dock says "Choice" and not "Target", and a player is one of the things offered.
                Trigger(view, "Comet, Stellar Pup", "0: Roll a six-sided die.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),

                    // The fixture board has no Comet to point at, so the ability on the stack is what names him.
                    Source = ViewHandle.None,
                    Text = "Comet deals damage equal to the number of loyalty counters on him to a creature or player.",
                    Targeted = false,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                        new TargetMessage { Player = Rival },
                    ],
                };

            case "choose-any-number":
                // Proliferate's "choose any number of permanents and/or players" (CR 701.34a): one choice of none, some
                // or all, chosen as the spell resolves — permanents and players with counters together.
                Trigger(view, "Tezzeret's Gambit", "Draw two cards, then proliferate.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = ViewHandle.None,
                    Text = "Proliferate: give each chosen permanent and player another counter of each kind already there.",
                    Targeted = false,
                    Optional = true,
                    AnyNumber = true,
                    Count = 3,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Player = Rival },
                    ],
                };

            case "choose-any-number-one":
                // The same choice with a single candidate: still asked, since choosing none is an answer.
                Trigger(view, "Tezzeret's Gambit", "Draw two cards, then proliferate.");
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = ViewHandle.None,
                    Text = "Proliferate: give each chosen permanent and player another counter of each kind already there.",
                    Targeted = false,
                    Optional = true,
                    AnyNumber = true,
                    Count = 1,
                    Candidates = [new TargetMessage { Player = Rival }],
                };

            case "choose-target-mode":
                // A modal spell's mode being aimed while the card is still in hand, so the cast can still be given up.
                Object(view, HandRitual).Name = "Kolaghan's Command";
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(HandRitual),
                    Source = new ViewHandle(HandRitual),
                    Text = "Kolaghan's Command deals 2 damage to any target.",
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(TheirSentinel) },
                    ],
                };

            case "choose-target-range":
                // A printed range, "one or two target creatures" (CR 601.2c): up to two, at least one.
                Object(view, HandRitual).Name = "Untimely Malfunction";
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(HandRitual),
                    Source = new ViewHandle(HandRitual),
                    Text = "One or two target creatures can't block this turn.",
                    Count = 2,
                    Minimum = 1,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(TheirSentinel) },
                    ],
                };

            case "divide-targets":
                // Fire Covenant's own "damage divided as you choose among any number of target creatures" (CR 601.2d):
                // the targets and their shares are chosen together, still in hand, so giving up is offered here too.
                Object(view, HandRitual).Name = "Fire Covenant";
                return new ChooseDividedTargetsPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(HandRitual),
                    Source = new ViewHandle(HandRitual),
                    Text = "Fire Covenant deals X damage divided as you choose among any number of target creatures.",
                    Total = 5,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(TheirSentinel) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                    ],
                };

            case "divide-targets-capped":
                Object(view, HandRitual).Name = "Shatterskull Smashing";
                return new ChooseDividedTargetsPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(HandRitual),
                    Source = new ViewHandle(HandRitual),
                    Text = "Shatterskull Smashing deals twice X damage divided among up to two target creatures and/or planeswalkers.",
                    Total = 12,
                    MaxTargets = 2,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Object = new ViewHandle(TheirSentinel) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                    ],
                };

            case "divide-targets-optional":
                // Court of Garenbrig's "distribute two +1/+1 counters among up to two target creatures" (CR 601.2c-d):
                // a trigger, so nothing to give up, and "up to" makes choosing no target at all a legal answer.
                Trigger(view, "Court of Garenbrig", "At the beginning of your upkeep, distribute two +1/+1 counters among up to two target creatures. Then if you're the monarch, double the number of +1/+1 counters on each creature you control.");
                return new ChooseDividedTargetsPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(OnStack),
                    Source = ViewHandle.None,
                    Text = "Distribute two +1/+1 counters among up to two target creatures.",
                    Total = 2,
                    MaxTargets = 2,
                    Optional = true,
                    Candidates =
                    [
                        new TargetMessage { Object = new ViewHandle(MyBrute) },
                        new TargetMessage { Object = new ViewHandle(MyLurker) },
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                    ],
                };

            case "unless-pays":
                Announce(view, "Gurmag Angler", "{2}{B}", string.Empty, Viewer);
                Counter(view, "Mana Leak");
                Float(view, ManaTypeView.Black, 3);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(TheirHandCard),
                    Subject = new ViewHandle(OnStack),
                    Verb = "Countered",
                    Question = "pay {cost}, or {subject} is countered?",
                    Yes = "Pay {cost}",
                    No = "Let it be countered",
                    Total = "{3}",
                    Remaining = "{0}",
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(TheirHandCard) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(TheirHandCard) },
                    ],
                };

            case "unless-pays-short":
                Announce(view, "Gurmag Angler", "{2}{B}", string.Empty, Viewer);
                Counter(view, "Mana Leak");
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(TheirHandCard),
                    Subject = new ViewHandle(OnStack),
                    Verb = "Countered",
                    Question = "pay {cost}, or {subject} is countered?",
                    Yes = "Pay {cost}",
                    No = "Let it be countered",
                    Total = "{3}",
                    Remaining = "{1}",
                    RemainingGeneric = 1,
                    Options =
                    [
                        Tap(MySwamp),
                        Tap(MySwampTwo),
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(TheirHandCard) },
                    ],
                };

            case "unless-pays-copy":
                // Chain Lightning's "that player may pay {R}{R}. If the player does, they may copy this spell".
                Announce(view, "Chain Lightning", "{R}", "Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy.", Rival, CardTypeView.Sorcery);
                Float(view, ManaTypeView.Red, 2);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = new ViewHandle(OnStack),
                    Verb = "CopySpell",
                    Question = "pay {cost} to copy {subject}?",
                    Detail = "You may choose a new target for the copy",
                    Yes = "Pay {cost}",
                    No = "Don't copy",
                    Total = "{R}{R}",
                    Remaining = "{0}",
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(OnStack) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            case "unless-pays-untap":
                // Mana Vault's "you may pay {4}. If you do, untap this artifact": a mid-resolution payment that buys
                // something rather than protecting it, so the dock says "to untap" where a counterspell's says
                // "or lose".
                ManaVault(view);
                Float(view, ManaTypeView.Black, 4);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(MyVault),
                    Subject = new ViewHandle(MyVault),
                    Verb = "UntapSource",
                    Question = "pay {cost} to untap {subject}?",
                    Yes = "Pay {cost}",
                    No = "Leave it tapped",
                    Total = "{4}",
                    Remaining = "{0}",
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(MyVault) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(MyVault) },
                    ],
                };

            case "unless-pays-return":
                // Decaying Soil's threshold ability: "you may pay {1}. If you do, return that card to your hand." The
                // payment buys back the card in the graveyard, so the dock says "to return" rather than "or lose".
                Announce(view, "Decaying Soil", "{1}{B}{B}", "Whenever a nontoken creature is put into your graveyard from the battlefield, you may pay {1}. If you do, return that card to your hand.", Viewer, CardTypeView.Enchantment);
                Float(view, ManaTypeView.Black, 1);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = new ViewHandle(MyGrave),
                    Verb = "ReturnToHand",
                    Question = "pay {cost} to return {subject} to your hand?",
                    Yes = "Pay {cost}",
                    No = "Leave it",
                    Total = "{1}",
                    Remaining = "{0}",
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(OnStack) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            case "unless-pays-return-gone":
                // The same offer once the card has left the graveyard (a Worldspine Wurm shuffled back first): paying is
                // still the player's choice, but it names no card and returns nothing.
                Announce(view, "Decaying Soil", "{1}{B}{B}", "Whenever a nontoken creature is put into your graveyard from the battlefield, you may pay {1}. If you do, return that card to your hand.", Viewer, CardTypeView.Enchantment);
                Float(view, ManaTypeView.Black, 1);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = ViewHandle.None,
                    Verb = "ReturnToHand",
                    Question = "pay {cost} to return {subject} to your hand?",
                    Yes = "Pay {cost}",
                    No = "Leave it",
                    Total = "{1}",
                    Remaining = "{0}",
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(OnStack) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            case "may":
                Announce(view, "Path to Exile", "{W}", "Exile target creature. Its controller may search their library for a basic land card.", Rival);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "SearchLibrary" };

            case "may-promise-gift":
                // Gift's optional additional cost (CR 702.174a), asked as the spell is announced.
                Announce(view, "Dawn's Truce", "{1}{W}", "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PromiseGiftCard" };

            case "may-promise-gift-tapped-fish":
                Announce(view, "Into the Flood Maw", "{U}", "Gift a tapped Fish (You may promise an opponent a gift as you cast this spell. If you do, they create a tapped 1/1 blue Fish creature token before its other effects.)", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PromiseGiftTappedFish" };

            // A shockland's "As this land enters, you may pay 2 life. If you don't, it enters tapped": life alone, no mana.
            case "unless-pays-life":
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandSwamp),
                    Subject = new ViewHandle(HandSwamp),
                    Verb = "EnterUntapped",
                    Question = "pay {cost}?",
                    Detail = "If you don't, it enters tapped",
                    Yes = "Pay {cost}",
                    No = "Enters tapped",
                    Total = "{0}",
                    Remaining = "{0}",
                    Life = 2,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(HandSwamp) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(HandSwamp) },
                    ],
                };

            // Lim-Dûl's Vault: the same life price bought over and over, with the cards it is about in the library
            // rather than in the dock, so the hint has to say where they can be seen.
            case "unless-pays-life-to-look-again":
                Announce(view, "Lim-Dûl's Vault", "{U}{B}",
                    "Look at the top five cards of your library. As many times as you choose, you may pay 1 life, put "
                        + "those cards on the bottom of your library in any order, then look at the top five cards of "
                        + "your library. Then shuffle and put the last cards you looked at this way on top in any order.",
                    Viewer);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = new ViewHandle(OnStack),
                    Verb = "LookAgain",
                    Question = "pay {cost} to look again?",
                    Detail = "The cards you are looking at go on the bottom  ·  click your library to see them",
                    Yes = "Pay {cost}",
                    No = "Keep these",
                    Total = "{0}",
                    Remaining = "{0}",
                    Life = 1,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(OnStack) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            // Master of Death: the card is the ability's source, in its owner's graveyard.
            case "unless-pays-life-to-return":
                Announce(view, "Master of Death", "{1}{U}{B}",
                    "At the beginning of your upkeep, if this card is in your graveyard, you may pay 1 life. If you "
                        + "do, return it to your hand.",
                    Viewer);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = new ViewHandle(OnStack),
                    Verb = "ReturnToHand",
                    Question = "pay {cost} to return {subject} to your hand?",
                    Detail = "It is in your graveyard",
                    Yes = "Pay {cost}",
                    No = "Leave it",
                    Total = "{0}",
                    Remaining = "{0}",
                    Life = 1,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(OnStack) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            // Ad Nauseam: Amount is what the card just revealed cost, since the next one's price is unknown.
            case "may-reveal-again":
                Announce(view, "Ad Nauseam", "{3}{B}{B}",
                    "Reveal the top card of your library and put that card into your hand. You lose life equal to its "
                        + "mana value. You may repeat this process any number of times.",
                    Viewer);
                return new MayPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Verb = "RevealAgain",
                    Amount = 3,
                };

            // Ripple 4 (CR 702.60a), as Thrumming Stone grants it: Amount is how many cards would be revealed.
            case "may-reveal-for-ripple":
                Announce(view, "Lightning Bolt", "{R}", "Lightning Bolt deals 3 damage to any target.", Viewer);
                return new MayPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Verb = "RevealForRipple",
                    Amount = 4,
                };

            case "unless-pays-life-unless-countered":
                // Ward — Pay 3 life (CR 702.21a): the caster who just targeted a warded permanent is the one asked,
                // not its controller — Sedgemoor Witch's shape, life rather than mana.
                Announce(view, "Lightning Bolt", "{R}", "Lightning Bolt deals 3 damage to any target.", Viewer);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(TheirHandCard),
                    Subject = new ViewHandle(OnStack),
                    Verb = "Countered",
                    Question = "pay {cost}, or {subject} is countered?",
                    Yes = "Pay {cost}",
                    No = "Let it be countered",
                    Total = "{0}",
                    Remaining = "{0}",
                    Life = 3,
                    Options =
                    [
                        new ActionOptionMessage { Kind = ActionKindView.PayCost, Source = new ViewHandle(TheirHandCard) },
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(TheirHandCard) },
                    ],
                };

            case "unless-pays-mana-and-life":
                // Zoraline, Cosmos Caller's "you may pay {W}{B} and 2 life. When you do, return target nonland permanent
                // card with mana value 3 or less from your graveyard to the battlefield": one payment, mana and life.
                Announce(view, "Zoraline, Cosmos Caller", "{1}{W}{B}", "Whenever Zoraline enters or attacks, you may pay {W}{B} and 2 life. When you do, return target nonland permanent card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it.", Viewer);
                return new UnlessPaysPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Subject = new ViewHandle(OnStack),
                    Verb = "ReturnPermanentCard",
                    Amount = 3,
                    Question = "pay {cost} to return a nonland permanent card with mana value 3 or less from your graveyard to the battlefield?",
                    Detail = "It returns with a finality counter on it",
                    Yes = "Pay {cost}",
                    No = "Don't pay",
                    Total = "{W}{B}",
                    Remaining = "{W}{B}",
                    RemainingColored = [ColorView.White, ColorView.Black],
                    Life = 2,
                    Options =
                    [
                        Tap(MySwamp),
                        new ActionOptionMessage { Kind = ActionKindView.DeclineCost, Source = new ViewHandle(OnStack) },
                    ],
                };

            case "may-discard-unless-countered":
                // Ward — Discard a card (CR 702.21a): Sunset Saboteur's shape, a card out of hand rather than life.
                Announce(view, "Lightning Bolt", "{R}", "Lightning Bolt deals 3 damage to any target.", Rival);
                return new MayPromptMessage { Player = Rival, Source = new ViewHandle(OnStack), Verb = "DiscardUnlessCountered", Amount = 1 };

            case "may-cast":
                Announce(view, "Bloodbraid Challenger", "{3}{R}{G}", "Cascade", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastWithoutPaying" };

            case "may-cast-transformed":
                // A Siege's defeat (CR 310.12b): exiled, then offered to its controller back face up, for nothing.
                Announce(view, "Invasion of Gobakhan", "{1}{W}", "When the last defense counter is removed from this permanent, exile it, then you may cast it transformed without paying its mana cost.", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastTransformed" };

            // Paradigm (CR 702.192a): the delayed ability offers a copy of the spell, which ceases to exist if declined.
            case "may-cast-paradigm-copy":
                Announce(view, "Germination Practicum", "{3}{G}{G}", "At the beginning of your precombat main phase, create a copy of this object in exile. You may cast the copy without paying its mana cost.", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastCopyWithoutPaying" };

            case "may-cast-exiled":
                Announce(view, "Chandra, Torch of Defiance", "{2}{R}{R}", "+1: Exile the top card of your library. You may cast that card. If you don't, Chandra deals 2 damage to each opponent.", Viewer);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastExiled" };

            // Malcolm, Alluring Scoundrel: the free cast is of the card just discarded, so declining leaves it in the
            // graveyard rather than sending it to the bottom of the library the way cascade's decline does.
            case "may-cast-discarded":
                Announce(view, "Malcolm, Alluring Scoundrel", "{1}{U}", "Whenever Malcolm deals combat damage to a player, put a chorus counter on it. Draw a card, then discard a card. If there are four or more chorus counters on Malcolm, you may cast the discarded card without paying its mana cost.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastDiscarded" };

            // "You may play that card" as an effect resolves (IEffectRuntime.PlayDuringResolution), with one way to play
            // it. The card here is one Shelldock Isle hid away face down: the dock names it as its chooser, who may look
            // at it, sees it (FaceDownName), not by its empty face-down name.
            case "may-play-land-during-resolution":
                Announce(view, "Shelldock Isle", string.Empty, "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it.", Viewer, CardTypeView.Land);
                return new MayPromptMessage { Player = Viewer, Source = HiddenAway(view, "Forest"), Verb = "PlayLandDuringResolution" };

            case "may-cast-during-resolution":
                Announce(view, "Shelldock Isle", string.Empty, "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it.", Viewer, CardTypeView.Land);
                return new MayPromptMessage { Player = Viewer, Source = HiddenAway(view, "Grizzly Bears"), Verb = "CastDuringResolution", Amount = 1 };

            case "may-cast-during-resolution-paid":
                Announce(view, "Djinn of Wishes", "{3}{U}{U}", "{2}{U}{U}, Remove a wish counter from this creature: Reveal the top card of your library. You may play that card without paying its mana cost. If you don't, exile it.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastDuringResolution", Amount = 0 };

            // Beseech the Mirror, bargained: the card it exiled face down is cast free, or else goes to its owner's hand.
            case "may-cast-found-or-to-hand":
                Announce(view, "Beseech the Mirror", "{1}{B}{B}{B}", "Bargain\nSearch your library for a card, exile it face down, then shuffle. If this spell was bargained, you may cast the exiled card without paying its mana cost if that spell's mana value is 4 or less. Put the exiled card into your hand if it wasn't cast this way.", Viewer, CardTypeView.Sorcery);
                return new MayPromptMessage { Player = Viewer, Source = HiddenAway(view, "Grizzly Bears"), Verb = "CastFoundOrPutIntoHand", Amount = 1 };

            // Several ways to play it — a modal double-faced card with a land back face — asked as one named choice.
            case "choose-play-way":
                Announce(view, "Shelldock Isle", string.Empty, "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it.", Viewer, CardTypeView.Land);
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Modes = ["Cast Sink into Stupor without paying its mana cost", "Play Soporific Springs as a land", "Don't play it"],
                    Eligible = [0, 1, 2],
                    Count = 1,
                    Heading = "You may play Sink into Stupor: choose how",
                };

            case "may-put-on-top":
                Announce(view, "Memory Lapse", "{1}{U}", "Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard.", Rival);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PutOnTop" };

            case "may-counter-or-cast":
                Announce(view, "Black Widow, Super Spy", string.Empty, "Whenever Black Widow deals combat damage to a player, that player exiles cards from the top of their library until they exile a nonland card. You may put a +1/+1 counter on Black Widow. If you don't, you may cast the exiled nonland card until end of turn and mana of any type can be spent to cast that spell.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PutCountersOrCast", Amount = 1 };

            case "may-copy":
                Announce(view, "Chain of Smog", "{1}{B}", "Target player discards two cards. That player may copy this spell and may choose a new target for that copy.", Rival);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "CopySpell", Amount = 1 };

            case "may-attach":
                // Cori-Steel Cutter's flurry: the Equipment has just made a Monk and asks whether to move onto it.
                Announce(view, "Cori-Steel Cutter", "{1}{R}", "Flurry — Whenever you cast your second spell each turn, create a 1/1 white Monk creature token with prowess. You may attach this Equipment to it.", Viewer, CardTypeView.Artifact);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "Attach" };

            case "may-return-and-attach":
                // Sword of the Meek's graveyard trigger: a 1/1 has entered and the Equipment may come back onto it.
                Announce(view, "Sword of the Meek", "{2}", "Whenever a 1/1 creature you control enters, you may return this card from your graveyard to the battlefield, then attach it to that creature.", Viewer, CardTypeView.Artifact);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "ReturnAndAttach" };

            case "may-create-treasure":
                // Generous Plunderer's upkeep: the optional half of a "you may ... When you do" reflexive trigger.
                Announce(view, "Generous Plunderer", "{1}{R}", "At the beginning of your upkeep, you may create a Treasure token. When you do, target opponent creates a tapped Treasure token.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "CreateTreasure" };

            case "may-shuffle-into-library" or "may-shuffle-no-cards":
                // Cathartic Parting's second sentence; Amount is how many graveyard cards are still legal targets.
                // With none, a yes still shuffles the library (CR 701.24c-d).
                Announce(view, "Cathartic Parting", "{1}{G}", "The owner of target artifact or enchantment an opponent controls shuffles it into their library. You may shuffle up to four target cards from your graveyard into your library.", Viewer, CardTypeView.Sorcery);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "ShuffleIntoLibrary", Amount = name == "may-shuffle-no-cards" ? 0 : 3 };

            case "may-draw":
                // Leovold's answer to being targeted: the whole question is whether the card is drawn, and the
                // count is what the sentence names, so a one-card draw reads "draw a card" rather than "draw".
                Announce(view, "Leovold, Emissary of Trest", string.Empty, "Whenever you or a permanent you control becomes the target of a spell or ability an opponent controls, you may draw a card.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "Draw", Amount = 1 };

            case "may-deal-damage":

                // The Sensational She-Hulk's answer to damage: the target was chosen as the ability went on the
                // stack, so the only question left is whether the damage happens.
                Announce(view, "The Sensational She-Hulk", string.Empty, "Whenever a creature you control is dealt damage, you may have The Sensational She-Hulk deal that much damage to any target. Do this only once each turn.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "DealDamage", Amount = 3 };

            case "may-explore-graveyard":
                // Explore (CR 701.44a): the revealed card was not a land, so the question is about that card, not the
                // Map token that asked it.
                Announce(view, "Map", string.Empty, "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.", Viewer, CardTypeView.Artifact);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(HandRitual), Verb = "PutIntoGraveyard" };

            case "may-put-on-bottom":
                // Jace, the Mind Sculptor's +2: the card looked at on top of a library is what the question names,
                // not the planeswalker that asked it.
                Announce(view, "Jace, the Mind Sculptor", "{2}{U}{U}", "+2: Look at the top card of target player's library. You may put that card on the bottom of that player's library.", Viewer, CardTypeView.Planeswalker);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(HandRitual), Verb = "PutOnBottom" };

            case "may-pay-energy":
                // "You may pay any amount of {E}" (CR 107.16, 122.1), asked one counter at a time. Amount is what is
                // still left to spend, so the dock reads the same on the first question and the last.
                Announce(view, "Galvanic Discharge", "{R}", "Choose target creature or planeswalker. You get {E}{E}{E} (three energy counters), then you may pay any amount of {E}. Galvanic Discharge deals that much damage to that permanent.", Viewer);
                view.Players[0].Energy = 3;
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PayEnergy", Amount = 3 };
            case "may-pay-energy-cost":
                // "You may pay {E}{E}{E}" (CR 107.16, 122.1): one yes or no for the whole printed amount, which is
                // only asked when the player has that much. Amount is what it costs, not what is left.
                Announce(view, "Guide of Souls", "{W}", "Whenever you attack, you may pay {E}{E}{E}. When you do, put two +1/+1 counters and a flying counter on target attacking creature. It becomes an Angel in addition to its other types.", Viewer, CardTypeView.Creature);
                view.Players[0].Energy = 4;
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "PayEnergyCost", Amount = 3 };

            case "may-protect-from-colorless":
                // "Protection from colorless or from the color of your choice" (CR 702.16a): a yes/no where neither
                // answer is a refusal — saying no asks for a color next, so neither button may read "No".
                Announce(view, "Giver of Runes", "{W}", "{T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "ProtectFromColorless" };

            case "may-choose-odd-or-even":
                // "As Gollum enters, choose odd or even" (CR 614.12a): the other yes/no where neither answer is a
                // refusal, and the source is the permanent entering rather than a spell on the stack.
                Announce(view, "Gollum, Riddle Master", "{1}{B}", "As Gollum enters, choose odd or even. (Zero is even.)", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "ChooseOddOrEven" };

            case "may-reveal-until-creature":
                // Oath of Druids (CR 701.20a): a "may" asked of the player whose upkeep it is, about an enchantment
                // somebody else controls — so the question names the source and the detail says what it costs.
                Announce(view, "Oath of Druids", "{1}{G}", "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.", Rival, CardTypeView.Enchantment);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "RevealUntilCreature" };

            case "may-reveal-until-shares-creature-type":
                // Heirloom Blade (CR 701.20a): the card found goes to hand and the rest to the bottom, not Oath's graveyard.
                Announce(view, "Heirloom Blade", "{3}", "Whenever equipped creature dies, you may reveal cards from the top of your library until you reveal a creature card that shares a creature type with it. Put that card into your hand and the rest on the bottom of your library in a random order.", Viewer, CardTypeView.Artifact);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "RevealUntilSharesCreatureType" };

            case "may-begin-on-battlefield":
                // An opening-hand action (CR 103.6a), asked before the first turn: the card is in the asked player's
                // own hand, so the question names it rather than anything on the stack.
                ObjectView caverns = Object(view, HandSwamp);
                caverns.Name = "Gemstone Caverns";
                caverns.Supertypes = [SupertypeView.Legendary];
                caverns.Subtypes = [];
                caverns.OracleText =
                    "If this card is in your opening hand and you're not the starting player, you may begin the game "
                    + "with Gemstone Caverns on the battlefield with a luck counter on it. If you do, exile a card "
                    + "from your hand.\n{T}: Add {C}. If Gemstone Caverns has a luck counter on it, instead add one "
                    + "mana of any color.";
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(HandSwamp), Verb = "BeginOnBattlefield" };

            case "may-reveal-for-miracle":
            case "may-cast-for-miracle-cost":
            {
                // Miracle (CR 702.94a). Two questions, one after the other and both about a card in the asked
                // player's own hand: the reveal as it is drawn, then the cast as the ability that reveal triggered
                // resolves. Drawn as the fixture's hand card so the dock names it the way a real one would.
                ObjectView fanatic = Object(view, HandSwamp);
                fanatic.Name = "Metamorphosis Fanatic";
                fanatic.Subtypes = [];
                fanatic.OracleText =
                    "Lifelink\nWhen this creature enters, return up to one target creature card from your graveyard "
                    + "to the battlefield with a lifelink counter on it.\nMiracle {1}{B} (You may cast this card for "
                    + "its miracle cost when you draw it if it's the first card you drew this turn.)";
                return new MayPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandSwamp),
                    Verb = name == "may-reveal-for-miracle" ? "RevealForMiracle" : "CastForMiracleCost",
                };
            }

            case "may-cast-for-madness-cost":
            {
                // Madness (CR 702.35a): the discarded card waits in exile while its linked ability resolves, and the
                // owner casts it for the madness cost or it goes to the graveyard. Drawn as the fixture's graveyard
                // card, renamed, so the dock names it the way a real one would.
                ObjectView ravager = Object(view, MyGrave);
                ravager.Name = "Anje's Ravager";
                ravager.OracleText =
                    "This creature attacks each combat if able.\nWhenever this creature attacks, discard your hand, "
                    + "then draw three cards.\nMadness {1}{R} (If you discard this card, discard it into exile. When "
                    + "you do, cast it for its madness cost or put it into your graveyard.)";
                Announce(view, "Anje's Ravager", "{2}{R}", "You may cast Anje's Ravager by paying {1}{R} rather than paying its mana cost. If you don't, put it into your graveyard.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "CastForMadnessCost" };
            }

            case "may-cast-for-rebound":
            {
                // Rebound (CR 702.88a): the spell waits in exile while its delayed ability resolves at the upkeep.
                // Drawn as the fixture's graveyard card, renamed, so the dock names it the way a real one would.
                ObjectView ephemerate = Object(view, MyGrave);
                ephemerate.Name = "Ephemerate";
                ephemerate.OracleText =
                    "Exile target creature you control, then return it to the battlefield under its owner's control.\n"
                    + "Rebound (If you cast this spell from your hand, exile it as it resolves. At the beginning of your "
                    + "next upkeep, you may cast this card from exile without paying its mana cost.)";
                Announce(view, "Ephemerate", "{W}", "At the beginning of your next upkeep, you may cast this card from exile without paying its mana cost.", Viewer, CardTypeView.Instant);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(MyGrave), Verb = "rebound.cast" };
            }

            case "may-remove-a-counter-to-destroy":
                // "You may remove a counter from her. When you do, destroy target artifact or enchantment that player
                // controls" (CR 118.3): declining costs the target, which is why neither button says "No".
                Announce(view, "Leatherhead, Swamp Stalker", string.Empty, "Whenever Leatherhead deals combat damage to a player, you may remove a counter from her. When you do, destroy target artifact or enchantment that player controls.", Viewer, CardTypeView.Creature);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "RemoveACounterToDestroy" };

            case "may-have-controller-draw":
            case "may-have-controller-draw-one":
                // Palantír of Orthanc: asked of the opponent it targeted, about a card its own controller would
                // draw. Declining is the other half of the card, so the detail prices it — and it is priced in the
                // singular too, since the first end step it ever reaches mills exactly one.
                Announce(view, "Palantír of Orthanc", "{3}", "At the beginning of your end step, put an influence counter on Palantír of Orthanc and scry 2. Then target opponent may have you draw a card. If that player doesn't, you mill X cards, where X is the number of influence counters on Palantír of Orthanc, and that player loses life equal to the total mana value of those cards.", Rival, CardTypeView.Artifact);
                return new MayPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Verb = "HaveControllerDraw",
                    Amount = name == "may-have-controller-draw-one" ? 1 : 4,
                };

            case "may-take-damage-instead-of-return":
                // Pia's Revolution: asked of the opponent it targeted, about an artifact card its controller would
                // get back. Neither button is primary, so passing through never spends the life.
                Announce(view, "Pia's Revolution", "{2}{R}", "Whenever a nontoken artifact is put into your graveyard from the battlefield, return that card to your hand unless target opponent has this enchantment deal 3 damage to them.", Rival, CardTypeView.Enchantment);
                return new MayPromptMessage { Player = Viewer, Source = new ViewHandle(OnStack), Verb = "TakeDamageInsteadOfReturn", Amount = 3 };

            case "separate-piles":
                Object(view, HandRitual).Name = "Fact or Fiction";
                return new ChooseTargetPromptMessage
                {
                    Player = Viewer,
                    Ability = new ViewHandle(HandRitual),
                    Source = new ViewHandle(HandRitual),
                    Text = "Separate into two piles: choose the first pile; the rest form the second. Either pile may be empty.",
                    Targeted = false,
                    Optional = true,
                    AnyNumber = true,
                    Count = 4,
                    Candidates = [.. RevealedLibraryTop(view, 4).Select(card => new TargetMessage { Object = card })],
                };

            case "choose-pile":
                Object(view, HandRitual).Name = "Fact or Fiction";
                RevealedLibraryTop(view, 4);
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes = ["First pile: Grizzly Bears, Island", "Second pile: Counterspell, Brainstorm"],
                    Eligible = [0, 1],
                    Count = 1,
                    Detail = "Chosen, not targeted",
                };

            case "modes-one":
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes =
                    [
                        "Kolaghan's Command deals 2 damage to any target",
                        "Return target creature card from your graveyard to your hand",
                        "Target player discards a card",
                        "Destroy target artifact",
                    ],
                    Eligible = [0, 1, 2, 3],
                    Count = 1,
                };

            case "additional-cost":
                // Bitter Triumph's "discard a card or pay 3 life": one of two costs, asked as it is cast.
                Object(view, HandRitual).Name = "Bitter Triumph";
                Object(view, HandRitual).ManaCost = "{1}{B}";
                Object(view, HandRitual).OracleText =
                    "As an additional cost to cast this spell, discard a card or pay 3 life.\nDestroy target creature or planeswalker.";
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes = ["Discard a card", "Pay 3 life"],
                    Eligible = [0, 1],
                    Count = 1,
                    AdditionalCost = true,
                };

            case "phyrexian-life":
                // Dismember's two {B/P} (CR 107.4f): which are paid with 2 life each is announced with the spell
                // (CR 601.2b), up to as many as the life total covers (CR 119.4), none at all a choice too.
                Object(view, HandRitual).Name = "Dismember";
                Object(view, HandRitual).ManaCost = "{1}{B/P}{B/P}";
                Object(view, HandRitual).OracleText =
                    "({B/P} can be paid with either {B} or 2 life.)\nTarget creature gets -5/-5 until end of turn.";
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes = ["Pay 2 life for {B/P}", "Pay 2 life for {B/P}"],
                    Eligible = [0, 1],
                    Affordable = [0],
                    Count = 0,
                    AtMost = 2,
                    AdditionalCost = true,
                    // The host's words for the choice (`phyrexian_heading`), true of a cost to attack too (Norn's Annex).
                    Heading = "Dismember: choose which {B/P} to pay with 2 life",
                    Detail = "Each one not chosen is paid with {B}",
                };

            case "venture-room":
                // Venturing on out of Forge (CR 309.5a): two arrows, so a choice, with no card to name as its source.
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = ViewHandle.None,
                    Modes =
                    [
                        "Trap! — Target player loses 5 life.",
                        "Arena — Goad target creature.",
                    ],
                    Eligible = [0, 1],
                    Count = 1,
                    Heading = "Venture into Undercity: choose the next room",
                };

            case "choose-option":
                // A named-option choice that is not a mode (CR 614.12a): Multiversal Passage's "choose a basic land
                // type", asked as the land enters, the way Painter's Servant's colour is. A heading like Venture's,
                // but with a card to name as its source, which is what tells the two apart in the dock.
                Announce(view, "Multiversal Passage", string.Empty, "As this land enters, choose a basic land type. Then you may pay 2 life. If you don't, it enters tapped.\nThis land is the chosen type.", Viewer, CardTypeView.Land);

                // Announce paints its caster's card; a land has no colour of its own.
                Object(view, OnStack).Colors.Clear();

                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Modes = ["Plains", "Island", "Swamp", "Mountain", "Forest"],
                    Eligible = [0, 1, 2, 3, 4],
                    Count = 1,
                    Heading = "Multiversal Passage: choose a basic land type",
                };

            case "remove-counters-up-to":
                // Heartless Act's "remove up to three counters from target creature" as it resolves (CR 608.2d, 122.1): the
                // counters on the rival's Serra Angel, of two kinds, any of them or none, up to three in all.
                Announce(view, "Heartless Act", "{1}{B}", "Choose one —\n• Destroy target creature with no counters on it.\n• Remove up to three counters from target creature.", Viewer);
                Object(view, TheirSentinel).Counters =
                [
                    new CounterView { Name = CounterNames.PlusOnePlusOne, Count = 2 },
                    new CounterView { Name = CounterNames.MinusOneMinusOne, Count = 2 },
                ];
                return new RemoveCountersPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Total = 3,
                    UpTo = true,
                    Pools =
                    [
                        new CounterPoolMessage { Object = new ViewHandle(TheirSentinel), Counter = CounterNames.PlusOnePlusOne, Count = 2 },
                        new CounterPoolMessage { Object = new ViewHandle(TheirSentinel), Counter = CounterNames.MinusOneMinusOne, Count = 2 },
                    ],
                };

            case "choose-option-word":
                // One of the words a card's text offers to choose between (CR 607.2f), asked as it enters (CR 614.12a):
                // the caption names every word, as the card does; the permanent keeps the one chosen.
                Announce(view, "Frostcliff Siege", "{1}{U}{R}", "As this enchantment enters, choose Jeskai or Temur.\n• Jeskai — Whenever one or more creatures you control deal combat damage to a player, draw a card.\n• Temur — Creatures you control get +1/+0 and have trample and haste.", Viewer, CardTypeView.Enchantment);
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Modes = ["Jeskai", "Temur"],
                    Eligible = [0, 1],
                    Count = 1,
                    Heading = "Frostcliff Siege: choose Jeskai or Temur",
                    Detail = "Chosen, not targeted",
                };

            case "choose-option-long":
                // A named-option choice too long for a button each (CR 205.3m's creature types, Banner of Kinship's
                // "choose a creature type"): the dock offers a box to type in and the few options that match it.
                Announce(view, "Banner of Kinship", "{5}", "As this artifact enters, choose a creature type. This artifact enters with a fate counter on it for each creature you control of the chosen type.\nCreatures you control of the chosen type get +1/+1 for each fate counter on this artifact.", Viewer, CardTypeView.Artifact);
                Object(view, OnStack).Colors.Clear();

                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Modes = [.. CreatureTypes],
                    Eligible = [.. Enumerable.Range(0, CreatureTypes.Count)],
                    Count = 1,
                    Heading = "Banner of Kinship: choose a creature type",
                };

            case "modes-many":
                // A card that repeats its modes rather than the one in hand, so the dock and the modes agree.
                Object(view, HandRitual).Name = "Fiery Confluence";
                Object(view, HandRitual).ManaCost = "{2}{R}";
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes =
                    [
                        "Fiery Confluence deals 1 damage to each opponent",
                        "Fiery Confluence deals 2 damage to target creature",
                        "Destroy target artifact",
                    ],
                    Eligible = [0, 1, 2],
                    Count = 3,
                    MayRepeat = true,
                };

            case "modes-range":
                // "Choose one or more" (escalate): the count is a range, so Done lights from one and the wording says
                // that the modes past the first cost something.
                Object(view, HandRitual).Name = "Collective Brutality";
                Object(view, HandRitual).ManaCost = "{1}{B}";
                Object(view, HandRitual).OracleText =
                    "Escalate—Discard a card.\nChoose one or more —\n• Target opponent reveals their hand. You choose an instant or sorcery card from it. That player discards that card.\n• Target creature gets -2/-2 until end of turn.\n• Target opponent loses 2 life and you gain 2 life.";
                return new ChooseModesPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Modes =
                    [
                        "Target opponent reveals their hand. You choose an instant or sorcery card from it. That player discards that card.",
                        "Target creature gets -2/-2 until end of turn.",
                        "Target opponent loses 2 life and you gain 2 life.",
                    ],
                    Eligible = [0, 1, 2],
                    Count = 1,
                    AtMost = 2,
                };

            case "order-triggers":
                // Endurance evoked: its enters trigger and evoke's sacrifice trigger went on the stack together, and
                // CR 603.3b gives its controller the order. Whichever is named last resolves first.
                Object(view, HandSwamp).Name = "Endurance";
                return new OrderTriggersPromptMessage
                {
                    Player = Viewer,
                    Sources = [new ViewHandle(HandSwamp), new ViewHandle(HandSwamp)],
                    Texts =
                    [
                        "When Endurance enters, up to one target player puts all the cards from their graveyard on the bottom of their library in a random order.",
                        "When Endurance enters, if its evoke cost was paid, its controller sacrifices it.",
                    ],
                };

            case "choose-replacement":
                // A shockland entering under an opponent's Archon of Emeria: two effects want to change how it
                // enters, and CR 616.1 gives its controller the order. Every option is legal.
                Object(view, HandSwamp).Name = "Hallowed Fountain";
                Object(view, HandSwamp).Supertypes = [];
                Object(view, HandSwamp).Subtypes = ["Plains", "Island"];
                Object(view, TheirSentinel).Name = "Archon of Emeria";
                return new ChooseReplacementPromptMessage
                {
                    Player = Viewer,
                    Subject = new ViewHandle(HandSwamp),
                    Event = "Hallowed Fountain enters the battlefield",
                    Sources = [new ViewHandle(HandSwamp), new ViewHandle(TheirSentinel)],
                    Hints = ["enters tapped unless you pay 2 life", "your opponents' nonbasic lands enter tapped"],
                };

            case "choose-replacement-unseen-source":
                // Ral Zarek's -7 made this player skip a turn and Ral went to the graveyard (CR 704.5i); a page reloaded since
                // holds no handle to Ral, so the prompt names it (SourceNames), beside Time Vault's "skip that turn instead".
                Object(view, MyLurker).Name = "Time Vault";
                return new ChooseReplacementPromptMessage
                {
                    Player = Viewer,
                    Event = "Your turn would begin",
                    Sources = [new ViewHandle(UnseenSource), new ViewHandle(MyLurker)],
                    Hints = ["you skip this turn", "skip this turn instead of untapping Time Vault"],
                    SourceNames = ["Ral Zarek, Guest Lecturer", "Time Vault"],
                };

            case "name-card":
                // Anointed Peacekeeper entering: the opponent's hand is laid out to click, and any other name is typed.
                Announce(view, "Anointed Peacekeeper", "{2}{W}", "Vigilance", Viewer, CardTypeView.Creature);
                return new NameCardPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Owner = Rival,
                    Hand = TheirHand(view),
                    Names = ["Anointed Peacekeeper", "Lightning Bolt", "Plains", "Wrath of God"],
                    Suggestion = "Wrath of God",
                    Kind = "a card name",
                };

            case "name-card-empty-hand":
                // Nothing to look at: the name is typed, or the suggestion taken.
                Announce(view, "Anointed Peacekeeper", "{2}{W}", "Vigilance", Viewer, CardTypeView.Creature);
                return new NameCardPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Owner = Rival,
                    Names = ["Anointed Peacekeeper", "Lightning Bolt", "Plains", "Wrath of God"],
                    Suggestion = "Anointed Peacekeeper",
                    Kind = "a card name",
                };

            case "choose-starting-player":
                // CR 103.1: this seat won the right to choose, and nobody has a hand yet.
                Opening(view, dealt: false);
                return new ChooseStartingPlayerPromptMessage { Player = Viewer, Candidates = [Viewer, Rival] };

            case "choose-starting-player-multiplayer":
                // Three players: "draw first" no longer names one player, so each other seat is a button of its own.
                Opening(view, dealt: false);
                view.Players.Add(new PlayerSummaryView { Seat = 3, Name = "Bot 3", Team = 3, Life = 20, LibraryCount = 40 });
                return new ChooseStartingPlayerPromptMessage { Player = Viewer, Candidates = [Viewer, Rival, 3] };

            case "mulligan":
                Opening(view, dealt: true);
                return new MulliganPromptMessage { Player = Viewer, Mulligans = 0, Keeps = 7 };

            case "mulligan-again":
                // A second hand of seven, of which keeping keeps six: the seventh goes on the bottom.
                Opening(view, dealt: true);
                return new MulliganPromptMessage { Player = Viewer, Mulligans = 1, Keeps = 6 };

            case "choose-put-on-bottom":
            {
                // Kept after two mulligans: two of the seven go on the bottom, chosen in the hand. The rules ask, not
                // a card, so there is no source to name.
                List<ViewHandle> hand = Opening(view, dealt: true);
                ChooseCardsPromptMessage bottom = Choose(CardChoicePurposeView.PutOnBottom, Viewer, hand, count: 2);
                bottom.Source = ViewHandle.None;
                return bottom;
            }

            case "choose-put-on-bottom-effect":
                // Valakut Awakening: any number of cards from your own hand on the bottom, so choosing none is an
                // answer, and the spell asking is named rather than a mulligan.
                return Choose(CardChoicePurposeView.PutOnBottom, Viewer, Hand(), count: 4, upTo: true);

            case "waiting-to-start":
                // The other seat is choosing who goes first; this one has no hand yet and nothing to do.
                Opening(view, dealt: false);
                return null;

            case "choose-player":
                // Baleful Mastery cast for {1}{B} at a three-player table: which opponent draws the card.
                Announce(view, "Baleful Mastery", "{3}{B}", "If the {1}{B} cost was paid, an opponent draws a card. Exile target creature or planeswalker.", Viewer);
                view.Players.Add(new PlayerSummaryView { Seat = 3, Name = "Bot 3", Team = 3, Life = 20, HandCount = 5, LibraryCount = 30 });
                return new ChoosePlayerPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Candidates = [Rival, 3],
                    Verb = "Draw",
                    Amount = 1,
                };

            case "choose-player-gift":
                // Gift at a three-player table (CR 702.174a): which opponent the gift is promised to.
                Announce(view, "Dawn's Truce", "{1}{W}", "Gift a card", Viewer);
                view.Players.Add(new PlayerSummaryView { Seat = 3, Name = "Bot 3", Team = 3, Life = 20, HandCount = 5, LibraryCount = 30 });
                return new ChoosePlayerPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Candidates = [Rival, 3],
                    Verb = "Gift",
                };

            case "choose-player-protection":
                Announce(view, "True-Name Nemesis", "{1}{U}{U}", "As this creature enters, choose a player. This creature has protection from the chosen player.", Viewer, CardTypeView.Creature);
                return new ChoosePlayerPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Candidates = [Viewer, Rival],
                    Verb = "Protection",
                };

            case "choose-player-protector":
                // A Siege entering at a three-player table: its controller chooses which opponent protects it (CR 310.12a).
                Announce(view, "Invasion of Gobakhan", "{1}{W}", "(As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)", Viewer);
                view.Players.Add(new PlayerSummaryView { Seat = 3, Name = "Bot 3", Team = 3, Life = 20, HandCount = 5, LibraryCount = 30 });
                return new ChoosePlayerPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Candidates = [Rival, 3],
                    Verb = "Protect",
                };

            case "choose-color":
                // One color, answered on the click: Painter's Servant's "as this creature enters, choose a color".
                Announce(view, "Painter's Servant", "{2}", "As this creature enters, choose a color.", Viewer, CardTypeView.Creature);
                return new ChooseColorsPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Choices = [.. Enum.GetValues<ColorView>()],
                    Count = 1,
                    Verb = "ChooseColor",
                };

            case "choose-color-protection":
                // The other half of Giver of Runes' one question: colorless was declined, so a color is chosen, and
                // the hint says what the creature gains rather than only that this is not a target.
                Announce(view, "Giver of Runes", "{W}", "{T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.", Viewer, CardTypeView.Creature);
                return new ChooseColorsPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Choices = [.. Enum.GetValues<ColorView>()],
                    Count = 1,
                    Verb = "GainProtection",
                };

            case "choose-colors-several":
                // Three charge counters removed, so three colors, and the same one may be taken more than once.
                Announce(view, "Coalition Relic", "{3}", "At the beginning of your precombat main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way.", Viewer, CardTypeView.Artifact);
                return new ChooseColorsPromptMessage
                {
                    Player = Viewer,
                    Source = new ViewHandle(OnStack),
                    Choices = [.. Enum.GetValues<ColorView>()],
                    Count = 3,
                    MayRepeat = true,
                    Verb = "AddMana",
                };

            case "search":
                return Search(view, cards: 9, count: 1, optional: true, SearchDestinationView.Hand);

            case "search-single-card":
                // A library down to its last card. The wording around a count of one is where "1 cards" hides.
                return Search(view, cards: 1, count: 1, optional: true, SearchDestinationView.Hand);

            case "search-several":
                return Search(view, cards: 9, count: 2, optional: false, SearchDestinationView.Battlefield);

            case "search-exile":
                // Ugin, Eye of the Storms: any number of colorless nonland cards, exiled face up.
                return Search(view, cards: 9, count: 9, optional: true, SearchDestinationView.Exile);

            case "search-exile-face-down":
                // Beseech the Mirror: one card, exiled face down for its searcher alone (CR 406.3).
                return Search(view, cards: 9, count: 1, optional: false, SearchDestinationView.ExileFaceDown);

            case "search-cast":
                // Sunforger: one instant with a stated quality, cast from the library as the ability resolves (CR 608.2g).
                return Search(view, cards: 9, count: 1, optional: true, SearchDestinationView.Cast);

            case "search-land-or-hand":
                // Archdruid's Charm: a creature or land card, the land put onto the battlefield tapped, any other into the hand.
                return Search(view, cards: 9, count: 1, optional: true, SearchDestinationView.LandTappedElseHand);

            case "choose-discard":
                return Choose(CardChoicePurposeView.Discard, Viewer, Hand(), count: 2);

            // Inti's and Formidable Speaker's "you may discard a card": one's own hand, and declining is an answer,
            // so the count reads singular and the button offers to discard nothing.
            case "choose-discard-up-to":
                return Choose(CardChoicePurposeView.Discard, Viewer, Hand(), count: 1, upTo: true);

            case "choose-from-their-hand":
                return Choose(CardChoicePurposeView.Discard, Rival, TheirHand(view), count: 1);

            // Revealing Eye: the same revealed hand, but "you may choose", so declining is an answer and the button
            // has to say so rather than offering to discard nothing in particular.
            case "choose-from-their-hand-up-to":
                return Choose(CardChoicePurposeView.Discard, Rival, TheirHand(view), count: 1, upTo: true);

            case "choose-put-on-top":
                return Choose(CardChoicePurposeView.PutOnTop, Viewer, Hand(), count: 2);

            case "choose-splice":
            {
                ChooseCardsPromptMessage choice = Choose(CardChoicePurposeView.Splice, Viewer, Hand(), count: 2, upTo: true);
                choice.Verdict.Addable = [.. choice.Cards.Take(2)];
                return choice;
            }

            case "choose-legend":
                return Choose(CardChoicePurposeView.KeepLegend, Viewer, [new ViewHandle(MyLurker), new ViewHandle(MyBrute)], count: 1);

            case "choose-sacrifice":
                return Choose(CardChoicePurposeView.Sacrifice, Viewer, [new ViewHandle(MyLurker), new ViewHandle(MyBrute)], count: 1);

            // Gut, True Soul Zealot: "you may sacrifice another creature or an artifact", an effect's own sacrifice
            // rather than a cost, so declining is an answer and the button has to say so instead of "Sacrifice 0".
            case "choose-sacrifice-up-to":
                return Choose(
                    CardChoicePurposeView.Sacrifice,
                    Viewer,
                    [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    count: 1,
                    upTo: true);

            // Balance: two of three lands to sacrifice, down to the fewest any player controls.
            case "choose-sacrifice-several":
                return Choose(
                    CardChoicePurposeView.Sacrifice,
                    Viewer,
                    [new ViewHandle(MySwamp), new ViewHandle(MySwampTwo), new ViewHandle(MySwampTapped)],
                    count: 2);

            case "choose-exile-cost":
                return Choose(CardChoicePurposeView.ExileToPay, Viewer, Hand(), count: 1);

            // Abhorrent Oculus: six cards out of a graveyard, not one out of a hand, so the dock names the zone and counts.
            case "choose-exile-graveyard":
                return Choose(CardChoicePurposeView.ExileToPay, Viewer, Graveyard(view, 8), count: 6);

            // Chrome Mox's imprint: the same own hand, but an effect taking a card rather than a cost being paid, and
            // "up to" one, so declining is a legal answer.
            case "choose-imprint":
                return Choose(CardChoicePurposeView.Exile, Viewer, Hand(), count: 1, upTo: true);

            // Jace Reawakened: the same own hand, up to one, but the card exiled becomes plotted (CR 702.170d) - a card
            // gained to cast free later, not one given up, and the caption says so.
            case "choose-plot":
                return Choose(CardChoicePurposeView.Plot, Viewer, Hand(), count: 1, upTo: true);

            case "choose-exile-hand":
                return Choose(CardChoicePurposeView.Exile, Rival, TheirHand(view), count: 1, upTo: true);

            case "choose-return-to-hand":
                return Choose(
                    CardChoicePurposeView.ReturnToHand,
                    Viewer,
                    [new ViewHandle(MySwamp), new ViewHandle(MySwampTwo), new ViewHandle(MySwampTapped)],
                    count: 1);

            case "choose-untap":
                return Choose(
                    CardChoicePurposeView.Untap,
                    Viewer,
                    [new ViewHandle(MySwamp), new ViewHandle(MySwampTwo), new ViewHandle(MySwampTapped), new ViewHandle(TheirPlains)],
                    count: 3,
                    upTo: true);

            case "choose-attach":
                // Battlefield Improvisation's "you may attach any number of Equipment you control to it": choosing
                // nothing attaches nothing.
                Equipment(view);
                return AnyNumber(Choose(
                    CardChoicePurposeView.Attach,
                    Viewer,
                    [new ViewHandle(MyKey), new ViewHandle(MyKeyTwo)],
                    count: 2,
                    upTo: true));

            case "choose-attach-one":
                // The same question with a single Equipment that could move: still "any number of", as the rules text says.
                Equipment(view);
                return AnyNumber(Choose(CardChoicePurposeView.Attach, Viewer, [new ViewHandle(MyKey)], count: 1, upTo: true));

            case "choose-tap-to-pay":
            {
                // Saddle 3 (CR 702.171a): any number of this seat's other untapped creatures, their power adding up to 3.
                Untap(view, MyLurker, MyBrute);
                List<ViewHandle> creatures = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)];
                ChooseCardsPromptMessage saddle = Choose(CardChoicePurposeView.TapToPay, Viewer, creatures, count: 2, upTo: true);
                saddle.MinimumPower = 3;
                saddle.Verdict.Complete = false;
                saddle.Powers = [.. creatures.Select(creature => Object(view, creature.Value).Power ?? 0)];
                return saddle;
            }

            case "choose-collect-evidence":
            {
                // Collect evidence 6 (CR 701.59a): any number of cards out of the graveyard, their mana values adding up to 6.
                List<ViewHandle> evidenceCards = Graveyard(view, 4);
                ChooseCardsPromptMessage evidence = Choose(CardChoicePurposeView.ExileToPay, Viewer, evidenceCards, count: 4, upTo: true);
                evidence.MinimumManaValue = 6;
                evidence.Verdict.Complete = false;
                evidence.ManaValues = [3, 0, 4, 1];
                return evidence;
            }

            case "choose-put-onto-battlefield":
                // Sneak Attack: up to one creature card from your own hand, so choosing nothing is declining.
                return Choose(CardChoicePurposeView.PutOntoBattlefield, Viewer, Hand(), count: 1, upTo: true);

            case "choose-put-onto-battlefield-revealed":
            {
                // Undercity's Throne of the Dead Three: the top of the library revealed, a creature card among them
                // put onto the battlefield, and one must be when any can.
                List<ViewHandle> revealed = Graveyard(view, 4);
                foreach (ViewHandle card in revealed)
                {
                    Object(view, card.Value).Zone = ZoneKindView.Library;
                }

                view.Zones.First(zone => zone.Kind == ZoneKindView.Graveyard && zone.Owner == Viewer).Contents.Clear();
                ChooseCardsPromptMessage choose = Choose(CardChoicePurposeView.PutOntoBattlefield, Viewer, revealed, count: 1, upTo: false);
                choose.Verdict.Addable = [revealed[2]];
                return choose;
            }

            case "choose-put-onto-battlefield-within-total":
            {
                // Michelangelo's Technique: the top of the library looked at, up to two creature cards among them with total mana
                // value 6 or less put onto the battlefield (CR 202.3).
                List<ViewHandle> looked = Graveyard(view, 4);
                foreach (ViewHandle card in looked)
                {
                    Object(view, card.Value).Zone = ZoneKindView.Library;
                }

                view.Zones.First(zone => zone.Kind == ZoneKindView.Graveyard && zone.Owner == Viewer).Contents.Clear();
                ChooseCardsPromptMessage choose = Choose(CardChoicePurposeView.PutOntoBattlefield, Viewer, looked, count: 2, upTo: true);
                choose.Kind = "creature";
                choose.Verdict.Addable = [looked[0], looked[1], looked[2]];
                choose.ManaValues = [4, 2, 3, 5];
                choose.Words = ["with total mana value 6 or less"];
                choose.MaximumTotalManaValue = 6;
                return choose;
            }

            case "choose-put-into-hand":
            {
                // Barrowgoyf: the cards it just milled, up to one creature card among them into your hand.
                List<ViewHandle> milled = Graveyard(view, 4);
                ChooseCardsPromptMessage choose = Choose(CardChoicePurposeView.PutIntoHand, Viewer, milled, count: 1, upTo: true);
                choose.Verdict.Addable = [milled[2]];
                return choose;
            }
            case "choose-put-into-their-hand":
            {
                // Karn, Scion of Urza's +1: the rival revealed the top two cards of their library and the viewer
                // picks which of them they get. The card is going into somebody else's hand, so the dock must not
                // offer to put it into the chooser's.
                List<ViewHandle> revealed = RevealedLibraryTop(view, 2);
                return Choose(CardChoicePurposeView.PutIntoHand, Rival, revealed, count: 1);
            }

            case "choose-cast":
                // Bilbo, Thief in the Night: up to one spell cast out of your own graveyard as the trigger resolves.
                return Choose(CardChoicePurposeView.Cast, Viewer, Graveyard(view, 4), count: 1, upTo: true);

            case "choose-keep":
                // Ajani, Nacatl Avenger's −4: one question per kind, "a creature" of the two, the rest sacrificed.
                ChooseCardsPromptMessage keep = Choose(
                    CardChoicePurposeView.Keep,
                    Viewer,
                    [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    count: 1);
                keep.Kind = "creature";
                return keep;

            case "choose-destroy":
                // Chaos Defiler's Battle Cannon: one of the opponent's nonland permanents, chosen rather than targeted.
                Planeswalker(view);
                ChooseCardsPromptMessage destroy = Choose(
                    CardChoicePurposeView.Destroy,
                    Rival,
                    [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel), new ViewHandle(TheirSaheeli)],
                    count: 1);
                destroy.Kind = "nonland permanent";
                return destroy;

            case "choose-vote":
            {
                // Council's Judgment: one vote for one of the nonland permanents this seat does not control
                // (CR 701.38a). Every player is asked the same question; what the tally does is the spell's business.
                Planeswalker(view);
                ChooseCardsPromptMessage vote = Choose(
                    CardChoicePurposeView.Vote,
                    Rival,
                    [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel), new ViewHandle(TheirSaheeli)],
                    count: 1);
                vote.Kind = "nonland permanent";
                return vote;
            }

            case "choose-put-into-graveyard":
            {
                // Currency Converter: one of the cards exiled with the artifact, put into its owner's graveyard. Not a
                // discard, so the dock must not offer to discard it.
                List<ViewHandle> exiled = Graveyard(view, 3);
                foreach (ViewHandle card in exiled)
                {
                    Object(view, card.Value).Zone = ZoneKindView.Exile;
                }

                view.Zones.First(zone => zone.Kind == ZoneKindView.Graveyard && zone.Owner == Viewer).Contents.Clear();
                return Choose(CardChoicePurposeView.PutIntoGraveyard, Viewer, exiled, count: 1);
            }

            case "choose-play":
            {
                // Dauthi Voidwalker: an exiled card an opponent owns, chosen to be granted a lasting permission to
                // play it this turn without paying its mana cost. The void counter is modelled as "exiled with this
                // creature" (CR 610.3), so the candidates are simply the rival's cards sitting in exile.
                List<ViewHandle> voided = ExiledByOpponent(view, 2);
                return Choose(CardChoicePurposeView.Play, Rival, voided, count: 1);
            }

            case "choose-nothing":
                // Nothing in the hand may be chosen, so the choice is empty and the dock only carries on.
                return new ChooseCardsPromptMessage
                {
                    Player = Viewer,
                    Owner = Viewer,
                    Source = new ViewHandle(HandRitual),
                    Purpose = CardChoicePurposeView.Discard,
                    Cards = Hand(),
                    Verdict = new() { Addable = [], Complete = true },
                    Count = 0,
                };

            case "choose-empty":
                return Choose(CardChoicePurposeView.Discard, Viewer, [], count: 0);

            case "choose-mixed-zones":
                return Choose(CardChoicePurposeView.Exile, Viewer,
                    [new ViewHandle(HandBrute), new ViewHandle(MyGrave)], count: 1, upTo: true);

            case "choose-return-effect":
                return Choose(CardChoicePurposeView.ReturnToHand, Viewer,
                    [new ViewHandle(MyLurker), new ViewHandle(MyBrute)], count: 1);

            case "choose-sacrifice-cost":
                return Choose(CardChoicePurposeView.Sacrifice, Viewer,
                    [new ViewHandle(MyLurker), new ViewHandle(MyBrute)], count: 1);

            case "choose-order-replacements":
                return Choose(CardChoicePurposeView.OrderSimultaneous, Viewer,
                    [new ViewHandle(MyLurker), new ViewHandle(MyBrute)], count: 1);

            case "distribute-empty":
                return Distribute(view, cards: 0, mayShuffle: false, spell: "Empty Library", text: "Look at the top card.",
                    places: [Place(CardDestinationKindView.LibraryTop)]);

            case "distribute-one":
                return Distribute(view, cards: 1, mayShuffle: false, spell: "One Card", text: "Put it on top or bottom.",
                    places: [Place(CardDestinationKindView.LibraryTop), Place(CardDestinationKindView.LibraryBottom)]);

            case "scry":
                return Distribute(view, cards: 2, mayShuffle: false, spell: "Preordain", text: "Scry 2. Draw a card.",
                    places: [Place(CardDestinationKindView.LibraryTop), Place(CardDestinationKindView.LibraryBottom)]);

            case "surveil":
                return Distribute(view, cards: 2, mayShuffle: false, spell: "Consider", text: "Surveil 1. Draw a card.",
                    places: [Place(CardDestinationKindView.LibraryTop), Place(CardDestinationKindView.Graveyard)]);

            case "arrange-in-any-order":
                return Distribute(view, cards: 3, mayShuffle: true, spell: "Ponder",
                    text: "Look at the top three cards of your library, then put them back in any order.",
                    places: [Place(CardDestinationKindView.LibraryTop)]);

            // Lim-Dûl's Vault: one pile and nothing to decide but the order, the mirror of Ponder's single top pile.
            case "arrange-onto-the-bottom":
                return Distribute(view, cards: 5, mayShuffle: false, spell: "Lim-Dûl's Vault",
                    text: "Look at the top five cards of your library. As many times as you choose, you may pay 1 life, "
                        + "put those cards on the bottom of your library in any order, then look at the top five cards "
                        + "of your library.",
                    places: [Place(CardDestinationKindView.LibraryBottom)]);

            // Stock Up: a destination that asks for an exact number, so the tally reads "0 of 2" until it is met.
            case "distribute-to-hand":
                return Distribute(view, cards: 5, mayShuffle: false, spell: "Stock Up",
                    text: "Look at the top five cards of your library. Put two of them into your hand and the rest on "
                        + "the bottom of your library in any order.",
                    places: [Place(CardDestinationKindView.Hand, 2, 2), Place(CardDestinationKindView.LibraryBottom)]);

            // The same question with too few cards to meet what it asks: the requirement has shrunk to one, which is
            // what the dock must show rather than an impossible "0 of 2".
            case "distribute-tight":
                return Distribute(view, cards: 1, mayShuffle: false, spell: "Stock Up",
                    text: "Look at the top five cards of your library. Put two of them into your hand and the rest on "
                        + "the bottom of your library in any order.",
                    places: [Place(CardDestinationKindView.Hand, 1, 2), Place(CardDestinationKindView.LibraryBottom)]);

            // Atraxa, Grand Unifier: a hand taking one card of each card type among those revealed, and a bottom in a
            // random order. The lands are one group and the sorceries the other, so only one of each gets a Hand button.
            case "distribute-each-card-type":
            {
                DistributeCardsPromptMessage prompt = Distribute(view, cards: 6, mayShuffle: false, spell: "Atraxa, Grand Unifier",
                    text: "When Atraxa enters, reveal the top ten cards of your library. For each card type, you may put a card "
                        + "of that type from among the revealed cards into your hand. Put the rest on the bottom of your library "
                        + "in a random order.",
                    places: [Place(CardDestinationKindView.Hand, 0, 2), Place(CardDestinationKindView.LibraryBottom)]);

                CardDestinationMessage hand = prompt.Destinations[0];
                CardDestinationMessage bottom = prompt.Destinations[1];
                bottom.Ordered = false;
                bottom.Randomized = true;

                List<ViewHandle> lands = [.. prompt.Cards.Where(card => view.Objects.First(o => o.Handle == card).CardTypes.Contains(CardTypeView.Land))];
                hand.Groups = [lands, [.. prompt.Cards.Except(lands)]];
                hand.GroupNames = ["Land", "Sorcery"];
                return prompt;
            }

            // Malevolent Rumble: a hand pile narrowed to some of the cards revealed (CardDestination.Eligible),
            // so only the permanent card among them carries a Hand button and the rest can only be buried.
            case "distribute-permanent-cards-only":
            {
                DistributeCardsPromptMessage prompt = Distribute(view, cards: 4, mayShuffle: false,
                    spell: "Malevolent Rumble",
                    text: "Reveal the top four cards of your library. You may put a permanent card from among them "
                        + "into your hand. Put the rest into your graveyard. Create a 0/1 colorless Eldrazi Spawn "
                        + "creature token with \"Sacrifice this token: Add {C}.\"",
                    places: [Place(CardDestinationKindView.Hand, 0, 1), Place(CardDestinationKindView.Graveyard)]);

                prompt.Destinations[0].Eligible = [.. prompt.Cards.Where(card =>
                    view.Objects.First(o => o.Handle == card).CardTypes.Contains(CardTypeView.Land))];
                return prompt;
            }

            // Manifest dread: one onto the battlefield face down, the other into the graveyard. The board carries a
            // face-down permanent on each side, so the picture shows both what its controller sees and what the other
            // player does.
            case "manifest-dread":
                FaceDownPermanents(view);
                return Distribute(view, cards: 2, mayShuffle: false, spell: "Abhorrent Oculus",
                    text: "At the beginning of each opponent's upkeep, manifest dread.",
                    places: [Place(CardDestinationKindView.BattlefieldFaceDown, 1, 1), Place(CardDestinationKindView.Graveyard)]);

            case "attackers":
                view.Step = TurnStepView.DeclareAttackers;
                Untap(view, MyLurker, MyBrute);
                return JudgedAttacks(new DeclareAttackersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    AttackablePlayers = [Rival],
                });

            case "attackers-planeswalker":
                // The rival has a planeswalker out, so each attacker may be sent at it instead (CR 508.1b).
                view.Step = TurnStepView.DeclareAttackers;
                Untap(view, MyLurker, MyBrute);
                Planeswalker(view);
                return JudgedAttacks(new DeclareAttackersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    AttackablePlayers = [Rival],
                    AttackablePlaneswalkers = [new ViewHandle(TheirSaheeli)],
                });

            case "attackers-battle":
                // The viewer's own Siege, protected by the rival (CR 310.12a), so each attacker may be sent at it
                // (CR 310.9b, 508.1b).
                view.Step = TurnStepView.DeclareAttackers;
                Untap(view, MyLurker, MyBrute);
                Siege(view);
                return JudgedAttacks(new DeclareAttackersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    AttackablePlayers = [Rival],
                    AttackableBattles = [new ViewHandle(MySiege)],
                });

            case "attackers-goaded":
                // The Brute was goaded (CR 701.15b): it must attack, and the dock waits until it does.
                view.Step = TurnStepView.DeclareAttackers;
                Untap(view, MyLurker, MyBrute);
                Object(view, MyBrute).Goaded = true;
                return JudgedAttacks(new DeclareAttackersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    AttackablePlayers = [Rival],
                    Requirements =
                    [
                        new AttackRequirementMessage { Attacker = new ViewHandle(MyBrute) },
                        new AttackRequirementMessage { Attacker = new ViewHandle(MyBrute), OtherThan = Viewer },
                    ],

                });

            case "blockers":
            case "blockers-restricted":
                UnderAttack(view);
                return Judged(new DeclareBlockersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    Attackers = [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel)],
                    MinimumBlockers = [1, 1],

                    // The restricted fixture holds a flier back out of the Lurker's reach (CR 702.9b), which is what
                    // makes some pairs offered and others not.
                    Allowed = name == "blockers"
                        ?
                        [
                            Block(MyLurker, TheirRecruit), Block(MyLurker, TheirSentinel),
                            Block(MyBrute, TheirRecruit), Block(MyBrute, TheirSentinel),
                        ]
                        :
                        [
                            Block(MyBrute, TheirRecruit),
                            Block(MyLurker, TheirSentinel), Block(MyBrute, TheirSentinel),
                        ],
                });

            case "blockers-one-at-most":
                // Savannah Lions can't be blocked by more than one creature (CR 509.1b): both of the viewer's creatures may
                // block it, but only one of them at a time.
                UnderAttack(view);
                return Judged(new DeclareBlockersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    Attackers = [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel)],
                    MinimumBlockers = [1, 1],
                    MaximumBlockers = [1, 0],
                    Allowed =
                    [
                        Block(MyLurker, TheirRecruit), Block(MyLurker, TheirSentinel),
                        Block(MyBrute, TheirRecruit), Block(MyBrute, TheirSentinel),
                    ],
                });

            case "blockers-two-each":
                // A creature that can block an additional creature each combat (CR 509.1a, Palace Guard's kind): the page holds
                // whatever blocks the rules' verdict offers - here the gallery's stand-in lets each creature block two - and
                // knows nothing of the rule itself.
                UnderAttack(view);
                return Judged(new DeclareBlockersPromptMessage
                {
                    Player = Viewer,
                    Eligible = [new ViewHandle(MyLurker), new ViewHandle(MyBrute)],
                    Attackers = [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel)],
                    MinimumBlockers = [1, 1],
                    Allowed =
                    [
                        Block(MyLurker, TheirRecruit), Block(MyLurker, TheirSentinel),
                        Block(MyBrute, TheirRecruit), Block(MyBrute, TheirSentinel),
                    ],
                }, BlocksPerBlocker("blockers-two-each"));

            case "assign-damage":
                // A trampling 4-power attacker gang-blocked by a 1/1 and a 1/4: its controller divides the 4 among the
                // blockers and, once both have lethal, the player (CR 510.1c, 702.19b).
                GangBlocked(view);
                return new AssignCombatDamagePromptMessage
                {
                    Player = Viewer,
                    Divisions =
                    [
                        new DamageDivisionMessage
                        {
                            Source = new ViewHandle(MyBrute),
                            Damage = 4,
                            Creatures = [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel)],
                            Lethal = [1, 4],
                            MayAssignToDefender = true,
                            Defender = Rival,
                            Suggested = [1, 3],
                            SuggestedToDefender = 0,
                        },
                    ],
                };

            default:
                throw new ArgumentOutOfRangeException(nameof(name), name, "Not a known prompt fixture.");
        }
    }

    // --- The position every fixture starts from ---

    /// <summary>
    /// A mid-game board seen from seat one: lands and two creatures each, a hand, a card in each graveyard.
    /// <para>
    /// Deliberately ordinary. The fixtures exist to picture the docks, and a board with something unusual on it
    /// would make every picture an argument about the board instead.
    /// </para>
    /// </summary>
    private static PlayerView Board()
    {
        var view = new PlayerView
        {
            Viewer = Viewer,
            Turn = 6,
            Step = TurnStepView.PrecombatMain,
            ActivePlayer = Viewer,
            PriorityPlayer = Viewer,
            Players =
            [
                new PlayerSummaryView { Seat = Viewer, Name = "You", Team = Viewer, Life = 17, HandCount = 4, LibraryCount = 26 },
                new PlayerSummaryView { Seat = Rival, Name = "Bot 2", Team = Rival, Life = 14, HandCount = 2, LibraryCount = 28 },
            ],
            Objects =
            [
                Land(MySwamp, "Swamp", Viewer),
                Land(MySwampTwo, "Swamp", Viewer),
                Land(MySwampTapped, "Swamp", Viewer, tapped: true),
                Body(MyLurker, "Vampire Nighthawk", "{B}", 1, 1, Viewer, ColorView.Black, "Zombie", tapped: true),
                Body(MyBrute, "Gurmag Angler", "{2}{B}", 3, 2, Viewer, ColorView.Black, "Zombie"),

                Land(TheirPlains, "Plains", Rival),
                Land(TheirPlainsTwo, "Plains", Rival),
                Body(TheirRecruit, "Savannah Lions", "{W}", 2, 1, Rival, ColorView.White, "Soldier"),
                Body(TheirSentinel, "Serra Angel", "{2}{W}", 1, 4, Rival, ColorView.White, "Soldier"),

                InHand(HandRitual, "Kolaghan's Command", "{1}{B}{R}", "Choose two —", CardTypeView.Instant, ColorView.Black),
                InHand(HandBrute, "Gurmag Angler", "{2}{B}", string.Empty, CardTypeView.Creature, ColorView.Black, 3, 2),
                InHand(HandTwist, "Mind Twist", "{X}{B}", "Target player discards X cards at random.", CardTypeView.Sorcery, ColorView.Black),
                InHand(HandSwamp, "Swamp", string.Empty, string.Empty, CardTypeView.Land, color: null),

                Buried(MyGrave, "Vampire Nighthawk", "{B}", 1, 1, Viewer, ColorView.Black),
                Buried(TheirGrave, "Savannah Lions", "{W}", 2, 1, Rival, ColorView.White),
            ],
        };

        view.Zones =
        [
            Visible(ZoneKindView.Battlefield, 0, [MySwamp, MySwampTwo, MySwampTapped, MyLurker, MyBrute, TheirPlains, TheirPlainsTwo, TheirRecruit, TheirSentinel]),
            Visible(ZoneKindView.Hand, Viewer, [HandRitual, HandBrute, HandTwist, HandSwamp]),
            Hidden(ZoneKindView.Hand, Rival, 2),
            Hidden(ZoneKindView.Library, Viewer, 26),
            Hidden(ZoneKindView.Library, Rival, 28),
            Visible(ZoneKindView.Graveyard, Viewer, [MyGrave]),
            Visible(ZoneKindView.Graveyard, Rival, [TheirGrave]),
            Visible(ZoneKindView.Stack, 0, []),
        ];

        return view;
    }

    /// <summary>
    /// The table before the first turn: nothing in play, nobody hurt, and once <paramref name="dealt"/> a hand of seven —
    /// the board's four and three more lands. Returns the hand.
    /// </summary>
    private static List<ViewHandle> Opening(PlayerView view, bool dealt)
    {
        int[] hand = dealt ? [HandRitual, HandBrute, HandTwist, HandSwamp, FirstOpening, FirstOpening + 1, FirstOpening + 2] : [];

        view.Turn = 0;
        view.Step = TurnStepView.Untap;
        view.PriorityPlayer = 0;
        view.Objects.RemoveAll(card => !hand.Contains(card.Handle.Value));

        if (dealt)
        {
            view.Objects.Add(InHand(FirstOpening, "Swamp", string.Empty, string.Empty, CardTypeView.Land, color: null));
            view.Objects.Add(InHand(FirstOpening + 1, "Swamp", string.Empty, string.Empty, CardTypeView.Land, color: null));
            view.Objects.Add(InHand(FirstOpening + 2, "Mountain", string.Empty, string.Empty, CardTypeView.Land, color: null));
        }

        foreach (PlayerSummaryView player in view.Players)
        {
            player.Life = 20;
            player.HandCount = hand.Length;
            player.LibraryCount = 40 - hand.Length;
        }

        view.Zones =
        [
            Visible(ZoneKindView.Battlefield, 0, []),
            Visible(ZoneKindView.Hand, Viewer, hand),
            Hidden(ZoneKindView.Hand, Rival, hand.Length),
            Hidden(ZoneKindView.Library, Viewer, 40 - hand.Length),
            Hidden(ZoneKindView.Library, Rival, 40 - hand.Length),
            Visible(ZoneKindView.Graveyard, Viewer, []),
            Visible(ZoneKindView.Graveyard, Rival, []),
            Visible(ZoneKindView.Stack, 0, []),
        ];

        return [.. hand.Select(handle => new ViewHandle(handle))];
    }

    private static List<ViewHandle> Hand() =>
        [new ViewHandle(HandRitual), new ViewHandle(HandBrute), new ViewHandle(HandTwist), new ViewHandle(HandSwamp)];

    /// <summary>
    /// The other player's hand, as a spell that reveals it makes it: the cards become objects this seat can see.
    /// </summary>
    private static List<ViewHandle> TheirHand(PlayerView view)
    {
        view.Objects.Add(InHand(TheirHandCard, "Wrath of God", "{2}{W}{W}", "Destroy all creatures. They can't be regenerated.", CardTypeView.Sorcery, ColorView.White, owner: Rival));
        view.Objects.Add(InHand(TheirHandCardTwo, "Plains", string.Empty, string.Empty, CardTypeView.Land, color: null, owner: Rival));

        Zone(view, ZoneKindView.Hand, Rival).Contents = [new ViewHandle(TheirHandCard), new ViewHandle(TheirHandCardTwo)];
        Zone(view, ZoneKindView.Hand, Rival).ContentsVisible = true;

        return [new ViewHandle(TheirHandCard), new ViewHandle(TheirHandCardTwo)];
    }

    // --- Changes a fixture makes to the board ---

    /// <summary>Puts a spell on the stack, cast by <paramref name="caster"/>.</summary>
    private static void Announce(
        PlayerView view,
        string name,
        string cost,
        string text,
        int caster,
        CardTypeView type = CardTypeView.Instant,
        int handle = OnStack)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(handle),
            Zone = ZoneKindView.Stack,
            Kind = ObjectKindView.Card,
            Owner = caster,
            Controller = caster,
            Name = name,
            ManaCost = cost,
            ManaValue = cost.Count(c => c == '{'),
            OracleText = text,
            CardTypes = [type],
            Colors = [caster == Viewer ? ColorView.Black : ColorView.White],
        });

        // Put on top of whatever was announced before it.
        ZoneView stack = Zone(view, ZoneKindView.Stack, 0);
        stack.Contents = [.. stack.Contents, new ViewHandle(handle)];
        stack.Count = stack.Contents.Count;

        if (caster != Viewer)
        {
            view.Players[1].HandCount--;
        }
    }

    /// <summary>
    /// Puts the other player's counterspell on the stack over what is already there, so an unless-pays dock can name
    /// the card asking as well as the spell at stake.
    /// </summary>
    private static void Counter(PlayerView view, string name)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(TheirHandCard),
            Zone = ZoneKindView.Stack,
            Kind = ObjectKindView.Card,
            Owner = Rival,
            Controller = Rival,
            Name = name,
            ManaCost = "{1}{U}",
            ManaValue = 2,
            OracleText = "Counter target spell unless its controller pays {3}.",
            CardTypes = [CardTypeView.Instant],
            Colors = [ColorView.Blue],
            Targets = [new SpellTargetView { Object = new ViewHandle(OnStack), StillLegal = true }],
        });

        Zone(view, ZoneKindView.Stack, 0).Contents.Add(new ViewHandle(TheirHandCard));
        Zone(view, ZoneKindView.Stack, 0).Count = 2;
        view.Players[1].HandCount--;
    }

    /// <summary>Puts a triggered ability of this seat's on the stack, which is what a target is being chosen for.</summary>
    private static void Trigger(PlayerView view, string source, string text)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(OnStack),
            Zone = ZoneKindView.Stack,
            Kind = ObjectKindView.AbilityOnStack,
            Owner = Viewer,
            Controller = Viewer,
            Name = source,
            OracleText = text,
            Colors = [ColorView.Black],
        });

        Zone(view, ZoneKindView.Stack, 0).Contents = [new ViewHandle(OnStack)];
        Zone(view, ZoneKindView.Stack, 0).Count = 1;
    }

    /// <summary>Puts mana in this seat's pool, so the rail shows chips and a payment dock has something to spend.</summary>
    private static void Float(PlayerView view, ManaTypeView type, int amount)
    {
        view.Players[0].Mana[(int)type] = amount;
        Object(view, MySwamp).Tapped = true;
        Object(view, MySwampTwo).Tapped = amount > 1;
    }

    /// <summary>Puts Saheeli, Sublime Artificer on the rival's battlefield with three loyalty (CR 306.5c).</summary>
    private static void Planeswalker(PlayerView view)
    {
        const string cost = "{1}{U/R}{U/R}";

        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(TheirSaheeli),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Rival,
            Controller = Rival,
            Name = "Saheeli, Sublime Artificer",
            ManaCost = cost,
            ManaValue = 3,
            OracleText = "Whenever you cast a noncreature spell, create a 1/1 colorless Servo artifact creature token.",
            Supertypes = [SupertypeView.Legendary],
            CardTypes = [CardTypeView.Planeswalker],
            Subtypes = ["Saheeli"],
            Colors = [ColorView.Blue, ColorView.Red],
            Counters = [new CounterView { Name = CounterNames.Loyalty, Count = 3 }],
        });

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(TheirSaheeli));
    }

    /// <summary>
    /// Puts Invasion of Gobakhan on the viewer's battlefield with two defense counters, protected by the rival
    /// (CR 310.4c, 310.12a).
    /// </summary>
    private static void Siege(PlayerView view)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(MySiege),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Viewer,
            Controller = Viewer,
            Name = "Invasion of Gobakhan",
            ManaCost = "{1}{W}",
            ManaValue = 2,
            OracleText = "(As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)\nWhen this Siege enters, look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.",
            CardTypes = [CardTypeView.Battle],
            Subtypes = ["Siege"],
            Colors = [ColorView.White],
            Counters = [new CounterView { Name = CounterNames.Defense, Count = 2 }],
            Protector = Rival,
        });

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MySiege));
    }

    /// <summary>Two untapped artifacts of the viewer's, for a fixture about something that taps artifacts.</summary>
    private static void Artifacts(PlayerView view)
    {
        foreach (int handle in (int[])[MyKey, MyKeyTwo])
        {
            view.Objects.Add(new ObjectView
            {
                Handle = new ViewHandle(handle),
                Zone = ZoneKindView.Battlefield,
                Kind = ObjectKindView.Card,
                Owner = Viewer,
                Controller = Viewer,
                Name = "Manifold Key",
                ManaCost = "{1}",
                ManaValue = 1,
                OracleText = "{1}, {T}: Untap another target artifact.\n{3}, {T}: Target creature can't be blocked this turn.",
                CardTypes = [CardTypeView.Artifact],
            });

            Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(handle));
        }
    }

    /// <summary>Two Equipment of the viewer's, for a fixture about attaching them.</summary>
    private static void Equipment(PlayerView view)
    {
        foreach (var (handle, name, text) in new[]
        {
            (MyKey, "Bonesplitter", "Equipped creature gets +2/+0.\nEquip {1}"),
            (MyKeyTwo, "Sword of War and Peace", "Equipped creature gets +2/+2 and has protection from red and from white.\nEquip {2}"),
        })
        {
            view.Objects.Add(new ObjectView
            {
                Handle = new ViewHandle(handle),
                Zone = ZoneKindView.Battlefield,
                Kind = ObjectKindView.Card,
                Owner = Viewer,
                Controller = Viewer,
                Name = name,
                OracleText = text,
                CardTypes = [CardTypeView.Artifact],
                Subtypes = ["Equipment"],
            });

            Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(handle));
        }
    }

    /// <summary>An untapped Chromatic Star of the viewer's, for the dock that asks which mana pays its own {1}.</summary>
    private static void ChromaticStar(PlayerView view)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(MyStar),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Viewer,
            Controller = Viewer,
            Name = "Chromatic Star",
            ManaCost = "{1}",
            ManaValue = 1,
            OracleText = "{1}, {T}, Sacrifice this artifact: Add one mana of any color.\n"
                + "When this artifact is put into a graveyard from the battlefield, draw a card.",
            CardTypes = [CardTypeView.Artifact],
        });

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyStar));
    }

    /// <summary>A tapped Mana Vault of the viewer's, for the dock that offers to untap it.</summary>
    private static void ManaVault(PlayerView view)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(MyVault),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Viewer,
            Controller = Viewer,
            Name = "Mana Vault",
            ManaCost = "{1}",
            ManaValue = 1,
            Tapped = true,
            OracleText = "This artifact doesn't untap during your untap step.\n"
                + "At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact.\n"
                + "At the beginning of your draw step, if this artifact is tapped, it deals 1 damage to you.\n"
                + "{T}: Add {C}{C}{C}.",
            CardTypes = [CardTypeView.Artifact],
        });

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyVault));
    }

    private static void Untap(PlayerView view, params int[] handles)
    {
        foreach (int handle in handles)
        {
            Object(view, handle).Tapped = false;
        }
    }

    /// <summary>The other player's creatures attacking this seat, with this seat's untapped to block.</summary>
    private static void UnderAttack(PlayerView view)
    {
        view.Turn = 7;
        view.Step = TurnStepView.DeclareBlockers;
        view.ActivePlayer = Rival;
        view.PriorityPlayer = 0;
        Untap(view, MyLurker, MyBrute);
        Object(view, TheirRecruit).Tapped = true;
        Object(view, TheirSentinel).Tapped = true;

        view.Combat.Attackers =
        [
            new AttackerView { Attacker = new ViewHandle(TheirRecruit), TargetKind = AttackTargetKindView.Player, TargetPlayer = Viewer },
            new AttackerView { Attacker = new ViewHandle(TheirSentinel), TargetKind = AttackTargetKindView.Player, TargetPlayer = Viewer },
        ];
    }

    /// <summary>This seat's attacker blocked by both of theirs, which is what an order is being chosen for.</summary>
    private static void GangBlocked(PlayerView view)
    {
        view.Step = TurnStepView.DeclareBlockers;
        Untap(view, MyBrute);
        Object(view, MyBrute).Tapped = true;

        view.Combat.Attackers =
        [
            new AttackerView
            {
                Attacker = new ViewHandle(MyBrute),
                TargetKind = AttackTargetKindView.Player,
                TargetPlayer = Rival,
                WasBlocked = true,
            },
        ];

        view.Combat.Blocks =
        [
            new BlockView
            {
                Attacker = new ViewHandle(MyBrute),
                Blockers = [new ViewHandle(TheirRecruit), new ViewHandle(TheirSentinel)],
            },
        ];
    }

    private static SearchLibraryPromptMessage Search(
        PlayerView view,
        int cards,
        int count,
        bool optional,
        SearchDestinationView destination)
    {
        string[] names = ["Swamp", "Dark Ritual", "Entomb", "Reanimate", "Thoughtseize", "Swamp", "Griselbrand", "Demonic Tutor", "Swamp"];
        string[] costs = [string.Empty, "{B}", "{B}", "{B}", "{B}", string.Empty, "{4}{B}{B}{B}{B}", "{1}{B}", string.Empty];

        List<ViewHandle> library = [];
        List<ViewHandle> eligible = [];

        for (int i = 0; i < cards; i++)
        {
            int handle = FirstLibraryCard + i;
            bool land = costs[i % costs.Length].Length == 0;

            view.Objects.Add(new ObjectView
            {
                Handle = new ViewHandle(handle),
                Zone = ZoneKindView.Library,
                ZoneOwner = Viewer,
                Kind = ObjectKindView.Card,
                Owner = Viewer,
                Controller = Viewer,
                Name = names[i % names.Length],
                ManaCost = costs[i % costs.Length],
                CardTypes = [land ? CardTypeView.Land : CardTypeView.Sorcery],
                Supertypes = land ? [SupertypeView.Basic] : [],
                Subtypes = land ? ["Swamp"] : [],
                Colors = land ? [] : [ColorView.Black],
            });

            library.Add(new ViewHandle(handle));

            // A search is for something in particular more often than not, so most fixtures dim the rest.
            if (land || destination is SearchDestinationView.Hand or SearchDestinationView.ExileFaceDown or SearchDestinationView.Cast)
            {
                eligible.Add(new ViewHandle(handle));
            }
        }

        Zone(view, ZoneKindView.Library, Viewer).Count = cards;

        return new SearchLibraryPromptMessage
        {
            Player = Viewer,
            Count = count,
            CountKind = optional ? CountKindView.UpTo : CountKindView.AsManyAsCan,
            Library = library,
            Verdict = new() { Addable = eligible, Complete = optional },
            Destination = destination,
        };
    }

    /// <summary>One destination of a distribution, taking any number of cards unless told otherwise.</summary>
    private static CardDestinationMessage Place(CardDestinationKindView kind, int min = 0, int max = int.MaxValue) =>
        new()
        {
            Kind = kind,
            Owner = Viewer,
            Min = min,
            Max = max,
            Ordered = kind is CardDestinationKindView.LibraryTop or CardDestinationKindView.LibraryBottom,
        };

    /// <summary>
    /// Cards looked at and given a destination each: scry, surveil, Ponder, Stock Up. Every card is eligible
    /// everywhere, which is what every card of this shape does so far.
    /// </summary>
    private static DistributeCardsPromptMessage Distribute(
        PlayerView view,
        int cards,
        bool mayShuffle,
        string spell,
        string text,
        List<CardDestinationMessage> places)
    {
        string[] names = ["Entomb", "Swamp", "Griselbrand"];
        string[] costs = ["{B}", string.Empty, "{4}{B}{B}{B}{B}"];

        List<ViewHandle> looked = [];
        for (int i = 0; i < cards; i++)
        {
            int handle = FirstLookedAt + i;
            bool land = costs[i % costs.Length].Length == 0;

            view.Objects.Add(new ObjectView
            {
                Handle = new ViewHandle(handle),
                Zone = ZoneKindView.Library,
                ZoneOwner = Viewer,
                Kind = ObjectKindView.Card,
                Owner = Viewer,
                Controller = Viewer,
                Name = names[i % names.Length],
                ManaCost = costs[i % costs.Length],
                CardTypes = [land ? CardTypeView.Land : CardTypeView.Sorcery],
                Supertypes = land ? [SupertypeView.Basic] : [],
                Subtypes = land ? ["Swamp"] : [],
                Colors = land ? [] : [ColorView.Black],
            });

            looked.Add(new ViewHandle(handle));
        }

        foreach (CardDestinationMessage place in places)
        {
            place.Eligible = [.. looked];
        }

        Announce(view, spell, "{U}", text, Viewer, CardTypeView.Sorcery);

        return new DistributeCardsPromptMessage
        {
            Player = Viewer,
            Source = new ViewHandle(OnStack),
            Cards = looked,
            Destinations = places,
            MayShuffle = mayShuffle,
        };
    }

    /// <summary>Puts this many more cards into the viewer's graveyard and returns them, top last.</summary>
    private static List<ViewHandle> Graveyard(PlayerView view, int count)
    {
        string[] names = ["Brainstorm", "Island", "Grizzly Bears", "Lightning Bolt"];
        ZoneView graveyard = view.Zones.First(zone => zone.Kind == ZoneKindView.Graveyard && zone.Owner == Viewer);
        List<ViewHandle> buried = [];

        for (int i = 0; i < count; i++)
        {
            ObjectView card = Buried(FirstBuried + i, names[i % names.Length], "{U}", 2, 2, Viewer, ColorView.Blue);
            view.Objects.Add(card);
            graveyard.Contents.Add(card.Handle);
            buried.Add(card.Handle);
        }

        graveyard.Count = graveyard.Contents.Count;
        return buried;
    }

    /// <summary>The top of the rival's library, turned face up, as Karn, Scion of Urza's +1 reveals it.</summary>
    private static List<ViewHandle> RevealedLibraryTop(PlayerView view, int count)
    {
        string[] names = ["Grizzly Bears", "Counterspell", "Island", "Brainstorm"];
        List<ViewHandle> revealed = [];

        for (int i = 0; i < count; i++)
        {
            ObjectView card = Buried(FirstLibraryCard + i, names[i % names.Length], "{U}", 2, 2, Rival, ColorView.Blue);
            card.Zone = ZoneKindView.Library;
            view.Objects.Add(card);
            revealed.Add(card.Handle);
        }

        return revealed;
    }

    /// <summary>
    /// A card of the viewer's lying face down in exile, as hideaway leaves one (CR 406.3): nameless to the table, and
    /// known to the viewer, who may look at it.
    /// </summary>
    private static ViewHandle HiddenAway(PlayerView view, string name)
    {
        ObjectView card = Buried(FirstHidden, name, string.Empty, 0, 0, Viewer, ColorView.Blue);
        card.Zone = ZoneKindView.Exile;
        card.Name = string.Empty;
        card.FaceDown = true;
        card.FaceDownName = name;
        card.Known = true;
        card.CardId = name;
        view.Objects.Add(card);
        return card.Handle;
    }

    /// <summary>The rival's cards sitting in exile, as Dauthi Voidwalker's own would leave them (CR 610.3).</summary>
    private static List<ViewHandle> ExiledByOpponent(PlayerView view, int count)
    {
        string[] names = ["Counterspell", "Grizzly Bears", "Island", "Brainstorm"];
        List<ViewHandle> voided = [];

        for (int i = 0; i < count; i++)
        {
            ObjectView card = Buried(FirstVoided + i, names[i % names.Length], "{U}", 2, 2, Rival, ColorView.Blue);
            card.Zone = ZoneKindView.Exile;
            view.Objects.Add(card);
            voided.Add(card.Handle);
        }

        return voided;
    }

    /// <summary>
    /// A face-down 2/2 on each side (CR 708.2a): the viewer's knows what it is (CR 708.5), the rival's does not.
    /// </summary>
    private static void FaceDownPermanents(PlayerView view)
    {
        ZoneView battlefield = view.Zones.First(zone => zone.Kind == ZoneKindView.Battlefield);

        foreach ((int handle, int seat) in (ReadOnlySpan<(int, int)>)[(MyFaceDown, Viewer), (TheirFaceDown, Rival)])
        {
            view.Objects.Add(new ObjectView
            {
                Handle = new ViewHandle(handle),
                Zone = ZoneKindView.Battlefield,
                Kind = ObjectKindView.Card,
                Owner = seat,
                Controller = seat,
                Known = seat == Viewer,
                CardId = seat == Viewer ? "Tarmogoyf" : string.Empty,
                FaceDown = true,
                FaceDownName = seat == Viewer ? "Tarmogoyf" : string.Empty,
                CardTypes = [CardTypeView.Creature],
                Power = 2,
                Toughness = 2,
                DamageMarked = seat == Viewer ? 0 : 1,
            });

            battlefield.Contents.Add(new ViewHandle(handle));
        }

        battlefield.Count = battlefield.Contents.Count;
    }

    /// <summary>"Any number of" (the host's <c>AnyNumber</c>): every eligible card may be chosen, and the question says so.</summary>
    private static ChooseCardsPromptMessage AnyNumber(ChooseCardsPromptMessage choose)
    {
        choose.CountKind = CountKindView.AnyNumber;
        choose.Verdict.Complete = true;
        return choose;
    }

    private static ChooseCardsPromptMessage Choose(
        CardChoicePurposeView purpose,
        int owner,
        List<ViewHandle> cards,
        int count,
        bool upTo = false) =>
        new()
        {
            Player = Viewer,
            Owner = owner,
            Source = new ViewHandle(HandRitual),
            Purpose = purpose,
            Cards = cards,
            Verdict = new() { Addable = [.. cards], Complete = upTo || count == 0 },
            Count = count,
            CountKind = upTo ? CountKindView.UpTo : CountKindView.Exactly,
        };

    /// <summary>
    /// Celestial Colonnade, animated: a Land Creature — Elemental of 4/4, white and blue, with the ability that made
    /// it one still on offer. The point of the picture is that nothing special draws it — the client reads computed
    /// card types, colors and power straight off the view — and that the ability's printed text fits its menu row.
    /// </summary>
    private static PriorityPromptMessage AnimatedLand(PlayerView view)
    {
        const string animate =
            "{3}{W}{U}: Until end of turn, this land becomes a 4/4 white and blue Elemental creature with flying and "
            + "vigilance. It's still a land.";

        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(MyColonnade),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Viewer,
            Controller = Viewer,
            Name = "Celestial Colonnade",
            OracleText = $"This land enters tapped.\n{{T}}: Add {{W}} or {{U}}.\n{animate}",
            CardTypes = [CardTypeView.Land, CardTypeView.Creature],
            Subtypes = ["Elemental"],
            Colors = [ColorView.White, ColorView.Blue],
            Power = 4,
            Toughness = 4,
        });

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Insert(3, new ViewHandle(MyColonnade));

        PriorityPromptMessage priority = Priority();
        priority.Options.Insert(3, new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(MyColonnade),
            ProducesOfAnyOneColor = 1,
            ColorChoices = [ColorView.White, ColorView.Blue],
            Description = "{T}: Add {W} or {U}.",
        });

        priority.Options.Insert(4, new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(MyColonnade),
            Affordable = true,
            Description = animate,
        });

        return priority;
    }

    /// <summary>Urza with two untapped artifacts, one action option per possible tap payment.</summary>
    private static PriorityPromptMessage TapForMana(PlayerView view)
    {
        view.Objects.RemoveAll(o => o.Zone == ZoneKindView.Battlefield && o.Controller == Viewer);
        var battlefield = Zone(view, ZoneKindView.Battlefield, 0);
        battlefield.Contents.RemoveAll(h => h.Value is MySwamp or MySwampTwo or MySwampTapped or MyLurker or MyBrute);
        var urza = new ObjectView
        {
            Handle = new ViewHandle(MyLurker), Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card, Owner = Viewer, Controller = Viewer,
            Name = "Urza, Lord High Artificer", ManaCost = "{2}{U}{U}",
            CardTypes = [CardTypeView.Creature], Colors = [ColorView.Blue], Power = 1, Toughness = 4,
            OracleText = "Tap an untapped artifact you control: Add {U}.",
        };
        view.Objects.Add(urza);
        battlefield.Contents.Add(urza.Handle);
        var priority = new PriorityPromptMessage { Player = Viewer };
        foreach (var (id, name) in new[] { (MyKey, "Sol Ring"), (MyKeyTwo, "Construct") })
        {
            var artifact = new ObjectView
            {
                Handle = new ViewHandle(id), Zone = ZoneKindView.Battlefield,
                Kind = ObjectKindView.Card, Owner = Viewer, Controller = Viewer,
                Name = name, CardTypes = [CardTypeView.Artifact],
            };
            view.Objects.Add(artifact);
            battlefield.Contents.Add(artifact.Handle);
            priority.Options.Add(new ActionOptionMessage
            {
                Kind = ActionKindView.ActivateAbility, Source = urza.Handle, Tap = artifact.Handle,
                CostText = "Tap an untapped artifact you control", Produces = [ManaTypeView.Blue],
                Description = "Tap an untapped artifact you control: Add {U}.",
            });
        }
        return priority;
    }

    /// <summary>
    /// Squandered Resources beside a Swamp and a Strip Mine: "Sacrifice a land: Add one mana of any type the
    /// sacrificed land could produce." The server offers one option per candidate's own colors and, separately, one
    /// for a candidate that could only add colorless, which says so with a choice and no colours in it.
    /// </summary>
    private static PriorityPromptMessage SacrificeForMana(PlayerView view)
    {
        view.Objects.Add(new ObjectView
        {
            Handle = new ViewHandle(MyResources),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = Viewer,
            Controller = Viewer,
            Name = "Squandered Resources",
            ManaCost = "{B}{G}",
            OracleText = "Sacrifice a land: Add one mana of any type the sacrificed land could produce.",
            CardTypes = [CardTypeView.Enchantment],
            Colors = [ColorView.Black, ColorView.Green],
        });

        ObjectView stripMine = Land(MyStripMine, "Strip Mine", Viewer);
        stripMine.Supertypes = [];
        stripMine.Subtypes = [];
        stripMine.OracleText = "{T}: Add {C}.\n{T}, Sacrifice this land: Destroy target land.";
        view.Objects.Add(stripMine);

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyResources));
        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyStripMine));

        const string text = "Sacrifice a land: Add 1 of any type the sacrificed permanent could produce.";
        PriorityPromptMessage priority = Priority();
        priority.Options.Add(new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(MyResources),
            ProducesOfAnyOneColor = 1,
            ColorChoices = [ColorView.Black],
            SacrificeCandidates = [new ViewHandle(MySwamp)],
            CostText = "Sacrifice a land",
            Description = text,
        });

        priority.Options.Add(new ActionOptionMessage
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(MyResources),
            ProducesOfAnyOneColor = 1,
            SacrificeCandidates = [new ViewHandle(MyStripMine)],
            CostText = "Sacrifice a land",
            Description = text,
        });

        return priority;
    }

    private static PriorityPromptMessage Priority() =>
        new()
        {
            Player = Viewer,
            Options =
            [
                new ActionOptionMessage { Kind = ActionKindView.PlayLand, Source = new ViewHandle(HandSwamp) },
                Tap(MySwamp),
                Tap(MySwampTwo),
                new ActionOptionMessage
                {
                    Kind = ActionKindView.CastSpell,
                    Source = new ViewHandle(HandBrute),
                    Affordable = true,
                },
                new ActionOptionMessage
                {
                    Kind = ActionKindView.CastSpell,
                    Source = new ViewHandle(HandTwist),
                    Affordable = true,
                    HasX = true,
                    MaxX = 1,
                    TargetsRequired = 1,
                    TargetCandidates = [new TargetMessage { Player = Rival }, new TargetMessage { Player = Viewer }],
                },
                new ActionOptionMessage
                {
                    Kind = ActionKindView.CastSpell,
                    Source = new ViewHandle(HandRitual),
                    TargetsRequired = 1,
                    TargetCandidates =
                    [
                        new TargetMessage { Object = new ViewHandle(TheirRecruit) },
                        new TargetMessage { Player = Rival },
                    ],
                },
            ],
        };

    /// <summary>
    /// Puts a dual land, an Island and a spell that wants both colours onto the board this fixture alone sees. Here
    /// rather than on the shared board, because a card added there would move every other fixture's picture.
    /// </summary>
    private static void DualLands(PlayerView view)
    {
        ObjectView tundra = Land(MyTundra, "Tundra", Viewer);
        tundra.Supertypes = [];
        tundra.Subtypes = ["Plains", "Island"];
        tundra.ProducesMana = true;

        ObjectView island = Land(MyIsland, "Island", Viewer);
        island.ProducesMana = true;

        view.Objects.Add(tundra);
        view.Objects.Add(island);
        view.Objects.Add(InHand(
            HandHerald,
            "Tidebound Herald",
            "{2}{U}{W}",
            "Flying. When this creature enters, draw a card.",
            CardTypeView.Creature,
            ColorView.Blue,
            2,
            3));

        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyTundra));
        Zone(view, ZoneKindView.Battlefield, 0).Contents.Add(new ViewHandle(MyIsland));
        Zone(view, ZoneKindView.Hand, Viewer).Contents.Add(new ViewHandle(HandHerald));
    }

    /// <summary>One of a land's mana abilities, named by the mana it adds.</summary>
    private static ActionOptionMessage TapFor(int land, ManaTypeView type) =>
        new()
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(land),
            Produces = [type],
            Description = $"{{T}}: Add {{{CardActions.Symbol(type)}}}.",
            CostText = "{T}",
        };

    private static ActionOptionMessage Tap(int land) =>
        new()
        {
            Kind = ActionKindView.ActivateAbility,
            Source = new ViewHandle(land),
            Produces = [ManaTypeView.Black],
            Description = "{T}: Add {B}.",
            CostText = "{T}",
        };

    private static BlockMessage Block(int blocker, int attacker) =>
        new() { Blocker = new ViewHandle(blocker), Attacker = new ViewHandle(attacker) };

    // --- Small builders ---

    private static ObjectView Land(int handle, string name, int seat, bool tapped = false) =>
        new()
        {
            Handle = new ViewHandle(handle),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = seat,
            Controller = seat,
            Name = name,
            CardTypes = [CardTypeView.Land],
            Supertypes = [SupertypeView.Basic],
            Subtypes = [name],
            Tapped = tapped,
        };

    private static ObjectView Body(
        int handle,
        string name,
        string cost,
        int power,
        int toughness,
        int seat,
        ColorView color,
        string subtype,
        bool tapped = false) =>
        new()
        {
            Handle = new ViewHandle(handle),
            Zone = ZoneKindView.Battlefield,
            Kind = ObjectKindView.Card,
            Owner = seat,
            Controller = seat,
            Name = name,
            ManaCost = cost,
            ManaValue = cost.Count(c => c == '{'),
            CardTypes = [CardTypeView.Creature],
            Subtypes = [subtype],
            Colors = [color],
            Power = power,
            Toughness = toughness,
            Tapped = tapped,
        };

    private static ObjectView Buried(int handle, string name, string cost, int power, int toughness, int seat, ColorView color) =>
        new()
        {
            Handle = new ViewHandle(handle),
            Zone = ZoneKindView.Graveyard,
            ZoneOwner = seat,
            Kind = ObjectKindView.Card,
            Owner = seat,
            Controller = seat,
            Name = name,
            ManaCost = cost,
            ManaValue = cost.Count(c => c == '{'),
            CardTypes = [CardTypeView.Creature],
            Colors = [color],
            Power = power,
            Toughness = toughness,
        };

    private static ObjectView InHand(
        int handle,
        string name,
        string cost,
        string text,
        CardTypeView type,
        ColorView? color,
        int? power = null,
        int? toughness = null,
        int owner = Viewer) =>
        new()
        {
            Handle = new ViewHandle(handle),
            Zone = ZoneKindView.Hand,
            ZoneOwner = owner,
            Kind = ObjectKindView.Card,
            Owner = owner,
            Controller = owner,
            Name = name,
            ManaCost = cost,
            ManaValue = cost.Count(c => c == '{'),
            OracleText = text,
            CardTypes = [type],
            Supertypes = type == CardTypeView.Land ? [SupertypeView.Basic] : [],
            Subtypes = type == CardTypeView.Land ? [name] : [],
            Colors = color is { } only ? [only] : [],
            Power = power,
            Toughness = toughness,
        };

    private static ZoneView Visible(ZoneKindView kind, int owner, int[] contents) =>
        new()
        {
            Kind = kind,
            Owner = owner,
            Count = contents.Length,
            Contents = [.. contents.Select(handle => new ViewHandle(handle))],
            ContentsVisible = true,
        };

    private static ZoneView Hidden(ZoneKindView kind, int owner, int count) =>
        new() { Kind = kind, Owner = owner, Count = count };

    /// <summary>How many attackers each creature may block in a canned declaration of blockers, as the gallery's stand-in judges it.</summary>
    public static int BlocksPerBlocker(string fixture) => fixture == "blockers-two-each" ? 2 : 1;

    /// <summary>A canned declaration of attackers with the stand-in's verdict on no attacks, as a table's prompt carries the host's.</summary>
    private static DeclareAttackersPromptMessage JudgedAttacks(DeclareAttackersPromptMessage prompt) => FixtureTransport.JudgeAttackers(prompt);

    /// <summary>A canned declaration of blockers with the stand-in's verdict on no blocks, as a table's prompt carries the host's.</summary>
    private static DeclareBlockersPromptMessage Judged(DeclareBlockersPromptMessage prompt, int perBlocker = 1)
    {
        prompt.Verdict = FixtureTransport.StandInBlockVerdict(prompt, [], perBlocker);
        return prompt;
    }

    private static ZoneView Zone(PlayerView view, ZoneKindView kind, int owner) =>
        view.Zones.First(zone => zone.Kind == kind && zone.Owner == owner);

    private static ObjectView Object(PlayerView view, int handle) =>
        view.Objects.First(o => o.Handle.Value == handle);
}

/// <summary>
/// A transport with the answer already in it: it hands the client one canned batch of server messages and swallows
/// everything sent back. Only ever built by the prompt gallery.
/// </summary>
public sealed class FixtureTransport : IGameTransport
{
    private IReadOnlyList<ServerMessage>? _pending;
    private readonly List<PromptMessage> _prompts;
    private readonly List<ServerMessage> _verdicts = [];

    private readonly int _perBlocker;

    /// <param name="messages">The canned batch.</param>
    /// <param name="perBlocker">How many attackers each creature may block, for the stand-in's verdicts on blocks.</param>
    public FixtureTransport(IReadOnlyList<ServerMessage> messages, int perBlocker = 1)
    {
        ArgumentNullException.ThrowIfNull(messages);
        _pending = messages;
        _prompts = [.. messages.OfType<PromptMessage>()];
        _perBlocker = perBlocker;
    }

    public bool IsConnected => true;

    /// <summary>What the dock sent, oldest first, for a test to read back what a click answered.</summary>
    public List<ClientMessage> Sent { get; } = [];

    /// <summary>
    /// Nothing is listening: a click in the gallery has nowhere to go and no game to change, and is only kept. A check of a
    /// choice of cards is answered as the gallery's stand-in for the rules (<see cref="StandInVerdict"/>), so its dock can be
    /// clicked through.
    /// </summary>
    public void Send(ClientMessage message)
    {
        Sent.Add(message);
        if (message is CheckSelectionMessage check
            && _prompts.FirstOrDefault(prompt => prompt.RequestId == check.RequestId)?.Prompt is ISelectionPrompt asked)
        {
            _verdicts.Add(new SelectionVerdictMessage { RequestId = check.RequestId, Selected = check.Selected, Verdict = StandInVerdict(asked, check.Selected) });
        }

        if (message is CheckAttacksMessage attacks
            && _prompts.FirstOrDefault(prompt => prompt.RequestId == attacks.RequestId)?.Prompt is DeclareAttackersPromptMessage attacking)
        {
            _verdicts.Add(new AttacksVerdictMessage { RequestId = attacks.RequestId, Attacks = attacks.Attacks, Verdict = StandInAttackVerdict(attacking, attacks.Attacks) });
        }

        if (message is CheckBlocksMessage blocks
            && _prompts.FirstOrDefault(prompt => prompt.RequestId == blocks.RequestId)?.Prompt is DeclareBlockersPromptMessage declaring)
        {
            _verdicts.Add(new BlocksVerdictMessage { RequestId = blocks.RequestId, Blocks = blocks.Blocks, Verdict = StandInBlockVerdict(declaring, blocks.Blocks, _perBlocker) });
        }
    }

    /// <summary>
    /// The gallery has no rules engine: a declaration of attackers of a canned prompt is judged by counting its
    /// <see cref="DeclareAttackersPromptMessage.Requirements"/> as CR 508.1d does - each attacker's best attack free of cost,
    /// added up, is the most a declaration must obey. Only ever the gallery's and the client tests'; a table asks the host.
    /// </summary>
    public static AttackVerdictView StandInAttackVerdict(DeclareAttackersPromptMessage asked, IReadOnlyList<AttackMessage> attacks)
    {
        ArgumentNullException.ThrowIfNull(asked);
        ArgumentNullException.ThrowIfNull(attacks);
        bool Same(AttackMessage a, AttackMessage b) => a.TargetPlayer == b.TargetPlayer && a.TargetPermanent == b.TargetPermanent;
        int Gain(AttackMessage attack) => asked.Requirements.Count(r => Obeys(r, attack));
        int Best(ViewHandle creature, bool freeOnly) =>
            asked.AttacksFor(creature).Where(attack => !freeOnly || asked.CostOf(attack) is null).Select(Gain).DefaultIfEmpty(0).Max();
        int most = asked.Eligible.Sum(creature => Best(creature, true));
        bool structural = attacks.All(attack => asked.Eligible.Contains(attack.Attacker) && asked.AttacksFor(attack.Attacker).Exists(a => Same(a, attack)))
            && attacks.Select(attack => attack.Attacker).Distinct().Count() == attacks.Count;
        int obeyed = attacks.Sum(Gain);
        List<ViewHandle> free = [.. asked.Eligible.Where(creature => !attacks.Any(attack => attack.Attacker == creature))];
        int rest = free.Sum(creature => Best(creature, false));
        List<AttackMessage> addable = structural
            ? [.. free.SelectMany(creature => asked.AttacksFor(creature).Where(attack => obeyed + Gain(attack) + rest - Best(creature, false) >= most))]
            : [];
        ViewHandle shortOf = structural && obeyed < most
            ? asked.Eligible.FirstOrDefault(creature => (attacks.FirstOrDefault(a => a.Attacker == creature) is { } at ? Gain(at) : 0) < Best(creature, true))
            : ViewHandle.None;
        return new AttackVerdictView
        {
            Complete = structural && obeyed >= most,
            Addable = addable,
            Free = most == 0,
            Issue = !structural
                ? new AttackIssueView { Creature = attacks.FirstOrDefault()?.Attacker ?? ViewHandle.None, Words = "can't attack", Rule = "508.1c" }
                : !shortOf.IsNone ? new AttackIssueView { Creature = shortOf, Words = "must attack", Rule = "508.1d" } : null,
        };
    }

    /// <summary>Whether <paramref name="attack"/> obeys the requirement <paramref name="r"/> (CR 508.1d), as the stand-in counts.</summary>
    private static bool Obeys(AttackRequirementMessage r, AttackMessage attack) =>
        r.Attacker == attack.Attacker
        && (r.TargetPermanent.IsNone
            ? r.TargetPlayer == 0 || (attack.TargetPermanent.IsNone && attack.TargetPlayer == r.TargetPlayer)
            : attack.TargetPermanent == r.TargetPermanent)
        && (r.OtherThan == 0 || (attack.TargetPermanent.IsNone && attack.TargetPlayer != r.OtherThan));

    /// <summary>
    /// A canned declaration of attackers judged as the host would carry it: the most its requirements ask, the stand-in's own
    /// declaration obeying it (each creature at its best free attack) and the verdict on no attacks. The gallery's and the
    /// client tests'.
    /// </summary>
    public static DeclareAttackersPromptMessage JudgeAttackers(DeclareAttackersPromptMessage prompt)
    {
        ArgumentNullException.ThrowIfNull(prompt);
        prompt.Suggested = [];
        prompt.MostRequirements = 0;
        foreach (ViewHandle creature in prompt.Eligible)
        {
            AttackMessage? best = null;
            int gain = 0;
            foreach (AttackMessage attack in prompt.AttacksFor(creature).Where(attack => prompt.CostOf(attack) is null))
            {
                int g = prompt.Requirements.Count(r => Obeys(r, attack));
                if (g > gain)
                {
                    (best, gain) = (attack, g);
                }
            }

            if (best is not null)
            {
                prompt.Suggested.Add(best);
                prompt.MostRequirements += gain;
            }
        }

        prompt.Verdict = StandInAttackVerdict(prompt, []);
        return prompt;
    }

    /// <summary>
    /// The gallery has no rules engine: a declaration of a canned prompt is judged by its allowed pairs, each attacker's fewest
    /// and most blockers, and how many attackers each creature may block (<paramref name="perBlocker"/>), with no requirements.
    /// Only ever the gallery's; a table asks the host.
    /// </summary>
    public static BlockVerdictView StandInBlockVerdict(DeclareBlockersPromptMessage asked, IReadOnlyList<BlockMessage> blocks, int perBlocker = 1)
    {
        ArgumentNullException.ThrowIfNull(asked);
        ArgumentNullException.ThrowIfNull(blocks);
        bool Allowed(BlockMessage block) => asked.Allowed.Exists(pair => pair.Blocker == block.Blocker && pair.Attacker == block.Attacker);
        bool Has(BlockMessage pair) => blocks.Any(block => block.Blocker == pair.Blocker && block.Attacker == pair.Attacker);
        int On(ViewHandle attacker) => blocks.Count(block => block.Attacker == attacker);
        int By(ViewHandle blocker) => blocks.Count(block => block.Blocker == blocker);
        bool complete = blocks.All(Allowed)
            && blocks.Select(block => (block.Blocker, block.Attacker)).Distinct().Count() == blocks.Count
            && blocks.All(block => By(block.Blocker) <= perBlocker)
            && asked.Attackers.Select((attacker, i) => (On(attacker), i)).All(a => a.Item1 == 0 || (a.Item1 >= asked.MinimumFor(a.i) && a.Item1 <= asked.MaximumFor(a.i)));
        List<BlockMessage> addable =
        [
            .. asked.Allowed.Where(pair => !Has(pair) && By(pair.Blocker) < perBlocker
                && On(pair.Attacker) < asked.MaximumFor(asked.Attackers.IndexOf(pair.Attacker))),
        ];
        return new BlockVerdictView { Complete = complete, Addable = addable };
    }

    /// <summary>
    /// The gallery has no rules engine: a selection of a canned choice is judged by its count alone, among the cards its
    /// prompt lets be picked first. Only ever the gallery's; a table asks the host.
    /// </summary>
    public static SelectionVerdictView StandInVerdict(ISelectionPrompt asked, IReadOnlyList<ViewHandle> selected)
    {
        ArgumentNullException.ThrowIfNull(asked);
        ArgumentNullException.ThrowIfNull(selected);
        bool among = selected.Distinct().Count() == selected.Count && selected.All(asked.Verdict.Addable.Contains);
        int n = selected.Count;
        bool room = asked.CountKind == CountKindView.AnyNumber || n < asked.Count;
        List<ViewHandle> addable = room ? [.. asked.Verdict.Addable.Where(card => !selected.Contains(card))] : [];
        bool complete = among && asked.CountKind switch
        {
            CountKindView.Exactly => n == asked.Count,
            CountKindView.AsManyAsCan => n == asked.Count || addable.Count == 0,
            CountKindView.AnyNumber => true,
            _ => n <= asked.Count,
        };

        // A cost's totals and craft's least, which the canned prompt carries for its caption.
        if (asked is ChooseCardsPromptMessage cards)
        {
            int Sum(List<int> values) => selected.Sum(card => cards.Cards.IndexOf(card) is >= 0 and var i && i < values.Count ? values[i] : 0);
            complete &= n >= cards.Minimum && Sum(cards.Powers) >= cards.MinimumPower && Sum(cards.ManaValues) >= cards.MinimumManaValue;
        }

        return new SelectionVerdictView { Complete = complete, Addable = addable };
    }

    public IReadOnlyList<ServerMessage> Receive()
    {
        List<ServerMessage> waiting = [.. _pending ?? [], .. _verdicts];
        _pending = null;
        _verdicts.Clear();
        return waiting;
    }

    public void Dispose()
    {
    }
}
