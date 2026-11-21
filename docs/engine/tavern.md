# Tavern activations (world 15)

How the tavern world (`TAVERN.RGM`, world 15) offers its three conversation and exit prompts. The condition grammar, `PlayerDistance`, `PlayerLooking` and `PlayerArc` are defined in [SOUP scripting](SOUP.md); the prompt mechanism is the [activation prompt](hud.md#activation-prompt).

Comparison byte 2 is `<` and conjunction byte 1 is `and` (see [Comparison Operators](SOUP.md#comparison-operators)). `PlayerDistance` is the sum of absolute X, Y and Z differences in runtime units (256 per world unit), so each limit below is a Manhattan distance.

## Entry points

| Actor | Prompt key (packed value) | Offered when |
|---|---|---|
| `TOBIAS` | `$ttt` (1953788964) | `PlayerDistance() < 65536`, local 3 = 0, flag `talking` = 0, `PlayerLooking(256)` = 1, flag `after_league` = 0 |
| `DREEKIUS`, first conversation | `$twb` (1651995684) | `PlayerDistance() < 75776`, local 4 = 0, `PlayerArc(1024, 600)` = 1, flag `talking` = 0, `PlayerLooking(600)` = 1, local 10 = 1, flag `DreekiusName` = 0 |
| `DREEKIUS`, repeat conversation | `$ttd` (1685353508) | the same conditions with flag `DreekiusName` = 1 |
| `EXTTAV` (tavern door) | `$exr` (1920492836) | `PlayerDistance() < 50688`, `PlayerLooking(456)` = 1 |

Each entry calls `ACTIVATE` with the key when its conditions hold, so the prompt appears and the activation fires on the use key.

Dreekius sets local 10 to 1 in its entry script before its first loop, so the conditions can hold only after that entry has run.

## Starting a conversation

Tobias, on activation: locks out the player (`lockoutPlayer(1)`), sets local 3 to 1, sets flag `talking` to 1 and flag `TobiasTalk` to 1. Its dialogue script then blocks on `PlayerLineUp(0, 0, 25600)` until the alignment completes.

Dreekius sets its attribute 44 to 0 whenever its conditions hold, before offering the prompt. On either activation it locks out the player, sets local 4 to 1 and flag `talking` to 1. Its dialogue script sets flags `DreekiusName` and `DreekiusTalk` to 1, which selects the repeat conversation afterwards.

## Leaving by the door

On activation the door clears flag `talking`, plays `FlatSound(35, 0, 0)` and requests `LoadWorld(timeofday, 8, 1724)`. The first argument is the current value of the number flag `timeofday`, the second is marker 8 and the third is the arrival heading 1724, in 2048-unit turns. The request is recorded and the main loop switches worlds after the current script step.
