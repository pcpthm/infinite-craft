# infinite-craft
# Infinite Craft solver

- Web requests and DB things are implemented in Python because I don't want to bloat Rust dependency.
- Search algorithm is implemented in Rust because I want performance. Rust programs runs the python program as a subprocess and communicates using stdio.

## Limited-depth enumeration

The goal is to enumerate all possible set (game state) of certain cardinality without repeating.
Such algorithm is described in literature as [an inductive algorithm used for polyomino enumeration](https://en.wikipedia.org/wiki/Polyomino#Inductive%20algorithms),

An efficient implemetation of such algorithm runs very fast and uses very little memory, as the algorithm doesn't maintain a "visited" set of states.
