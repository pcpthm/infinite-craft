# infinite-craft
# Infinite Craft solver

- Web requests and DB things are implemented in Python because I don't want to bloat Rust dependency.
- Search algorithm is implemented in Rust because I want performance. Rust programs runs the python program as a subprocess and communicates using stdio.

## Set enumeration

The goal is to enumerate, without repeating, all possible sets (game states) satisfying given constraints e.g. maximum cadinality.
Such algorithm is described in literatures as [Redelmeier's algorithm](https://www.sciencedirect.com/science/article/pii/S0012365X81800155) when used for [enumerating polyominos](https://en.wikipedia.org/wiki/Polyomino#Inductive_algorithms).

An efficient implemetation of the algorithm runs very fast and uses very little memory, as the algorithm doesn't maintain a "visited" set of states.
