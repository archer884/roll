# roll

> A dice roller.

Use expressions like `2d8+5`. Results are printed with the total first, by each individual roll, followed by the fixed modifier (if any). Max rolls (crits) are highlighted in green, while low rolls are highlighted in red.

## Installation

```
cargo install --git https://github.com/archer884/roll
```

You can get cargo [here](https://rustup.rs/).

## Saving rolls

Save a set of expressions with an alias. These expressions are stored in `~/.roll` in .json format. Only expressions that successfully compile will be stored.
```shell
$ roll add attack 1d20+4 2d6r+2 --comment "Two-handed sword; reroll 1s on damage"
$ roll attack
# attack
# Two-handed sword; reroll 1s on damage
  11  ::  1d20+4  ::   7  (+4)
   7  ::  2d6r+2  ::   2   3  (+2)     
```
Saving another roll with the same alias will overwrite the first.

## Listing rolls

```shell
$ roll list
# attack
# Two-handed sword; reroll 1s on damage
  1d20+4
  2d6r+2
```

## Deleting rolls

```shell
$ roll rm attack
$ roll list
```

## Repeating rolls

Any expression may be repeated by the addition of `*x` at the end, where x is some number. Note that some shells (zsh!) don't appreciate this, so you'll need to quote your expression, e.g. `"2d6*2"`.
```shell
$ roll 2d6*5
 8  ::  2d6  ::   5   3
 9  ::  2d6  ::   6   3
10  ::  2d6  ::   4   6
 6  ::  2d6  ::   2   4
 4  ::  2d6  ::   2   2
```
This also works for aliases.
