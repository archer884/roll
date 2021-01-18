# roll

> A dice roller.

Use expressions like 2d8+5. Results are printed with the total first, followed by
each individual roll, followed by the fixed modifier (if any). Max rolls (crits)
are highlighted in green, while low rolls are highlighted in red.

## Installation

```
cargo install --git https://github.com/archer884/roll
```

You can get cargo [here](https://rustup.rs/).

## Saving rolls

Save a set of expressions with an alias. These expressions are stored in
`~/.roll` in .json format. Only expressions that successfully compile will be
stored.
```shell
$ roll add attack 1d20+4 2d6+2
$ roll attack
attack
  1d20+4 = 16 :: 12 + 4
  2d6+2 = 11 :: 3 + 6 + 2
```
Saving another roll with the same alias will overwrite the first.

## Deleting rolls

```shell
$ roll rm attack
```

## Listing rolls

```shell
$ roll list
dex
  20+9
atk
  20+10
  10r+7
```

## Repeating rolls

Any expression may be repeated by the addition of `[x]` at the end, where x is some
number.
```shell
$ roll 2d6[5]
9 :: 6 + 3
11 :: 6 + 5
7 :: 6 + 1
7 :: 4 + 3
4 :: 2 + 2
```
This also works for aliases.
